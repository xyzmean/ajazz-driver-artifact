//! Reverse-engineered Ajazz HID protocol — Rust port of the `reverse` branch's src/protocol/core.ts.
//!
//! Wire format (see core.ts for the full spec):
//!   Request packet (32 bytes, sent with report id 0):
//!     [0] 0xAA  [1] cmd  [2] len  [3..4] addr (LE)  [6] last-packet flag  [8..] payload
//!   Response packet:
//!     [0] 0x55  [1] cmd  [2] lenOrType  [3..4] addr (LE)  [8..] payload
//!
//! HID transport is done here in Rust (hidapi) because Tauri webviews have no WebHID.

use hidapi::HidDevice;

pub const REPORT_ID: u8 = 0;
pub const REQUEST_HEADER: u8 = 0xAA;
pub const RESPONSE_HEADER: u8 = 0x55;
pub const HEADER_SIZE: usize = 8;
pub const REPORT_SIZE: usize = 32;
pub const USAGE_PAGES: &[u16] = &[0xFF60, 0xFF67, 0xFF68];

/// Command opcodes (subset of CMD from core.ts; extend as features are ported).
#[allow(dead_code)] // several opcodes are placeholders for not-yet-ported features
pub mod cmd {
    pub const GET_DEVICE_INFO: u8 = 16;
    pub const GET_GAME_MODE: u8 = 17;
    pub const GET_KEY: u8 = 18;
    pub const GET_LED_EFFECT: u8 = 19;
    pub const GET_MAGNETIC_AXIS_RT: u8 = 23;
    pub const SET_GAME_MODE: u8 = 33;
    pub const SET_KEY: u8 = 34;
    pub const SET_LED_EFFECT: u8 = 35;
    pub const SET_MAGNETIC_AXIS_RT: u8 = 39;
    pub const SET_FACTORY_RESET: u8 = 15;
    pub const SET_MUSIC_DATA: u8 = 53;
}

/// Build one 32-byte request packet (port of `buildPacket` / minified `P`).
pub fn build_packet(command: u8, len: u8, addr: u16, payload: Option<&[u8]>, last: bool) -> [u8; REPORT_SIZE] {
    let mut buf = [0u8; REPORT_SIZE];
    buf[0] = REQUEST_HEADER;
    buf[1] = command;
    buf[2] = len;
    buf[3] = (addr & 0xFF) as u8;
    buf[4] = ((addr >> 8) & 0xFF) as u8;
    buf[6] = if last { 1 } else { 0 };
    if let Some(p) = payload {
        let n = p.len().min(REPORT_SIZE - HEADER_SIZE);
        buf[HEADER_SIZE..HEADER_SIZE + n].copy_from_slice(&p[..n]);
    }
    buf
}

/// Write one packet (prefixed with report id 0, as hidapi requires).
fn send(device: &HidDevice, packet: &[u8; REPORT_SIZE]) -> Result<(), String> {
    let mut out = [0u8; REPORT_SIZE + 1];
    out[0] = REPORT_ID;
    out[1..].copy_from_slice(packet);
    device.write(&out).map_err(|e| format!("write failed: {e}"))?;
    Ok(())
}

/// Read a response packet matching `expected_cmd`, retrying reads until timeout.
fn recv(device: &HidDevice, expected_cmd: u8, timeout_ms: i32) -> Result<[u8; REPORT_SIZE], String> {
    let mut buf = [0u8; REPORT_SIZE];
    // A few reads to skip unrelated input reports (e.g. notifications).
    for _ in 0..8 {
        let n = device
            .read_timeout(&mut buf, timeout_ms)
            .map_err(|e| format!("read failed: {e}"))?;
        if n == 0 {
            return Err(format!("command 0x{expected_cmd:02x} response timeout"));
        }
        if buf[0] == RESPONSE_HEADER && buf[1] == expected_cmd {
            return Ok(buf);
        }
    }
    Err(format!("command 0x{expected_cmd:02x}: no matching response"))
}

/// Chunked read transport (port of `transfer`/`readDataChunks`/minified `C`).
/// Returns `content_size` reassembled payload bytes.
pub fn read_data(device: &HidDevice, command: u8, content_size: usize, timeout_ms: i32) -> Result<Vec<u8>, String> {
    let per_packet = REPORT_SIZE - HEADER_SIZE; // 24
    let packet_count = content_size.div_ceil(per_packet);
    let mut out: Vec<u8> = Vec::with_capacity(content_size);

    for i in 0..packet_count {
        let addr = (i * per_packet) as u16;
        let remaining = content_size - i * per_packet;
        let len = remaining.min(per_packet) as u8;
        let last = i == packet_count - 1;

        let packet = build_packet(command, len, addr, None, last);
        send(device, &packet)?;
        let resp = recv(device, command, timeout_ms)?;
        out.extend_from_slice(&resp[HEADER_SIZE..REPORT_SIZE]);
    }
    out.truncate(content_size);
    Ok(out)
}

/// Decoded GET_DEVICE_INFO payload (port of `getDeviceInfo` / minified `Ce`).
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub rom_size: u8,
    pub macro_space_size: u16,
    pub vendor_id: u16,
    pub product_id: u16,
    pub version: f32,
    pub sensor: u16,
    pub work_mode: u8,
    pub battery_level: u8,
    pub charge_status: u8,
    pub current_profile: u8,
    pub rt_precision: u8,
    pub frame_version: u8,
    pub lighting_version: u8,
}

pub fn parse_device_info(e: &[u8]) -> DeviceInfo {
    let u16le = |a: usize| e[a] as u16 | ((e[a + 1] as u16) << 8);
    let version = (((e[8] & 0x0F) as f32) + (((e[8] & 0xF0) >> 4) as f32) * 10.0 + (e[9] as f32) * 100.0) / 100.0;
    DeviceInfo {
        rom_size: e[0],
        macro_space_size: u16le(2),
        vendor_id: u16le(4),
        product_id: u16le(6),
        version: (version * 100.0).round() / 100.0,
        sensor: u16le(10),
        work_mode: e[16],
        battery_level: e[17],
        charge_status: e[18],
        current_profile: e[19],
        rt_precision: e[29],
        frame_version: e[30],
        lighting_version: e[31],
    }
}

/// Read & decode device info from an opened device.
pub fn get_device_info(device: &HidDevice) -> Result<DeviceInfo, String> {
    let payload = read_data(device, cmd::GET_DEVICE_INFO, 48, 500)?;
    Ok(parse_device_info(&payload))
}

/// Chunked write transport.
/// Splits `data` into packets of 24 bytes, sends each, and awaits a matching response confirmation.
pub fn write_data(device: &HidDevice, command: u8, data: &[u8], timeout_ms: i32) -> Result<(), String> {
    let content_size = data.len();
    let per_packet = REPORT_SIZE - HEADER_SIZE; // 24
    let packet_count = content_size.div_ceil(per_packet);

    for i in 0..packet_count {
        let addr = (i * per_packet) as u16;
        let from = i * per_packet;
        let to = (from + per_packet).min(content_size);
        let chunk = &data[from..to];
        let len = chunk.len() as u8;
        let last = i == packet_count - 1;

        let packet = build_packet(command, len, addr, Some(chunk), last);
        send(device, &packet)?;
        // Await confirmation response
        let _resp = recv(device, command, timeout_ms)?;
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameMode {
    pub game_mode: u8,
    pub fn_switch: u8,
    pub sleep_time: u8,
    pub key_delay: u8,
    pub report_rate: u8,
    pub system_mode: u8,
    pub tft_display_time: u8,
    pub top_dead_zone: f32,    // saved as value * 100 on wire
    pub bottom_dead_zone: f32, // saved as value * 100 on wire
    pub stability_mode: u8,
    pub auto_calibration: u8,
    pub single_key_wakeup: u8,
}

pub fn parse_game_mode(e: &[u8]) -> GameMode {
    GameMode {
        game_mode: e[1],
        fn_switch: e[2],
        sleep_time: e[3],
        key_delay: e[4],
        report_rate: e[5],
        system_mode: e[6],
        tft_display_time: e[7],
        top_dead_zone: (e[8] as f32) / 100.0,
        bottom_dead_zone: (e[9] as f32) / 100.0,
        stability_mode: e[11],
        auto_calibration: e[14],
        single_key_wakeup: e[15],
    }
}

pub fn encode_game_mode(v: &GameMode) -> [u8; 56] {
    let mut e = [0u8; 56];
    e[1] = v.game_mode;
    e[2] = v.fn_switch;
    e[3] = v.sleep_time;
    e[4] = v.key_delay;
    e[5] = v.report_rate;
    e[6] = v.system_mode;
    e[7] = v.tft_display_time;
    e[8] = (v.top_dead_zone * 100.0).round() as u8;
    e[9] = (v.bottom_dead_zone * 100.0).round() as u8;
    e[11] = v.stability_mode;
    e[14] = v.auto_calibration;
    e[15] = v.single_key_wakeup;
    e
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LedEffect {
    pub mode: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub driver_setting: u8,
    pub secondary_red: u8,
    pub secondary_green: u8,
    pub secondary_blue: u8,
    pub color_mode: u8,
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
    pub effect_mode_type: u8,
}

pub fn parse_led_effect(e: &[u8]) -> LedEffect {
    LedEffect {
        mode: e[0],
        red: e[1],
        green: e[2],
        blue: e[3],
        driver_setting: e[4],
        secondary_red: e[5],
        secondary_green: e[6],
        secondary_blue: e[7],
        color_mode: e[8],
        brightness: e[9],
        speed: e[10],
        direction: e[11],
        effect_mode_type: e[12],
    }
}

pub fn encode_led_effect(v: &LedEffect) -> [u8; 16] {
    let mut e = [0u8; 16];
    e[0] = v.mode;
    e[1] = v.red;
    e[2] = v.green;
    e[3] = v.blue;
    e[4] = 255; // driverSetting forced to 255 by upstream
    e[5] = v.secondary_red;
    e[6] = v.secondary_green;
    e[7] = v.secondary_blue;
    e[8] = v.color_mode;
    e[9] = v.brightness;
    e[10] = v.speed;
    e[11] = v.direction;
    e[12] = v.effect_mode_type;
    e[14] = 170; // checkCodeL
    e[15] = 85;  // checkCodeH
    e
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MagneticAxisRT {
    pub axis_type: u8,
    pub is_whole_fast: bool,
    pub is_rampage_mode: bool,
    pub trigger_key_stroke: f32,
    pub press_rt: f32,
    pub release_rt: f32,
}

pub fn parse_magnetic_rt(e: &[u8], rt_precision: u8) -> Vec<MagneticAxisRT> {
    let rt_scale = if rt_precision > 0 { 1000.0 } else { 100.0 };
    let stroke_scale = if rt_precision == 2 { 1000.0 } else { 100.0 };
    let mut out = Vec::with_capacity(128);
    for i in 0..128 {
        let b = i * 8;
        let flags = e[b + 1];
        let trigger_key_stroke = ((e[b + 2] as u16 | ((e[b + 3] as u16) << 8)) as f32) / stroke_scale;
        let press_rt = ((e[b + 4] as u16 | ((e[b + 5] as u16) << 8)) as f32) / rt_scale;
        let release_rt = ((e[b + 6] as u16 | ((e[b + 7] as u16) << 8)) as f32) / rt_scale;
        out.push(MagneticAxisRT {
            axis_type: e[b],
            is_whole_fast: (flags & 1) != 0,
            is_rampage_mode: (flags & 2) != 0,
            trigger_key_stroke,
            press_rt,
            release_rt,
        });
    }
    out
}

pub fn encode_magnetic_rt(v: &[MagneticAxisRT], rt_precision: u8) -> [u8; 1024] {
    let rt_scale = if rt_precision > 0 { 1000.0 } else { 100.0 };
    let stroke_scale = if rt_precision == 2 { 1000.0 } else { 100.0 };
    let mut e = [0u8; 1024];
    for i in 0..128.min(v.len()) {
        let b = i * 8;
        let item = &v[i];
        e[b] = item.axis_type;
        let mut flags = 0u8;
        if item.is_whole_fast { flags |= 1; }
        if item.is_rampage_mode { flags |= 2; }
        e[b + 1] = flags;
        
        let stroke = (item.trigger_key_stroke * stroke_scale).round() as u16;
        e[b + 2] = (stroke & 0xFF) as u8;
        e[b + 3] = ((stroke >> 8) & 0xFF) as u8;
        
        let press = (item.press_rt * rt_scale).round() as u16;
        e[b + 4] = (press & 0xFF) as u8;
        e[b + 5] = ((press >> 8) & 0xFF) as u8;
        
        let release = (item.release_rt * rt_scale).round() as u16;
        e[b + 6] = (release & 0xFF) as u8;
        e[b + 7] = ((release >> 8) & 0xFF) as u8;
    }
    e
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawKeyEntry {
    pub page_type: u8,
    pub param1: u8,
    pub param2: u8,
    pub param3: u8,
}

pub fn parse_key_data(e: &[u8]) -> Vec<RawKeyEntry> {
    let mut out = Vec::with_capacity(128);
    for i in 0..128 {
        let b = i * 4;
        if b + 4 <= e.len() {
            out.push(RawKeyEntry {
                page_type: e[b],
                param1: e[b + 1],
                param2: e[b + 2],
                param3: e[b + 3],
            });
        } else {
            out.push(RawKeyEntry {
                page_type: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            });
        }
    }
    out
}

pub fn encode_key_data(v: &[RawKeyEntry]) -> [u8; 512] {
    let mut e = [0u8; 512];
    for i in 0..128.min(v.len()) {
        let b = i * 4;
        let item = &v[i];
        e[b] = item.page_type;
        e[b + 1] = item.param1;
        e[b + 2] = item.param2;
        e[b + 3] = item.param3;
    }
    e
}

pub fn send_music_data(device: &HidDevice, music_type: u8, spectrum: &[u8]) -> Result<(), String> {
    let mut buf = [0u8; REPORT_SIZE];
    buf[0] = REQUEST_HEADER;
    buf[1] = cmd::SET_MUSIC_DATA;
    buf[2] = 0; // package ID
    buf[3] = music_type;
    
    // Copy spectrum amplitudes into buf[4..25] (up to 21 bytes)
    let n = spectrum.len().min(21);
    buf[4..4 + n].copy_from_slice(&spectrum[..n]);
    
    // Calculate checksum: buf[31] = sum(buf[0..30]) & 0xFF
    let mut sum: u32 = 0;
    for i in 0..31 {
        sum += buf[i] as u32;
    }
    buf[31] = (sum & 0xFF) as u8;
    
    // Send report
    let mut out = [0u8; REPORT_SIZE + 1];
    out[0] = REPORT_ID;
    out[1..].copy_from_slice(&buf);
    device.write(&out).map_err(|e| format!("write failed: {e}"))?;
    
    Ok(())
}

