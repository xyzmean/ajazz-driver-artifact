//! Tauri commands — the IPC surface the Vue frontend calls via invoke().

use crate::{models, protocol};
use hidapi::HidApi;
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub api: Mutex<HidApi>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product: Option<String>,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
}

/// List connected keyboards on our HID usage page (or any known model).
#[tauri::command]
pub fn list_devices(state: tauri::State<AppState>) -> Result<Vec<DeviceSummary>, String> {
    let mut api = state.api.lock().map_err(|e| e.to_string())?;
    api.refresh_devices().map_err(|e| e.to_string())?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for d in api.device_list() {
        let vid = d.vendor_id();
        let pid = d.product_id();
        let model = models::find(vid, pid);

        let usage_page = d.usage_page();
        let usage = d.usage();
        let interface = d.interface_number();

        if protocol::USAGE_PAGES.contains(&usage_page) && usage == 97 {
            eprintln!(
                "MATCHED HID DEVICE: VID={:04x}, PID={:04x}, Model={:?}, Path={:?}, UsagePage={:04X}, Usage={}, Interface={}",
                vid, pid, model, d.path(), usage_page, usage, interface
            );
        }

        // 0xFF68 is the primary USB control interface, 0xFF60 is the primary 2.4G control interface.
        // 0xFF67 is a secondary light/sync endpoint and should not be used as the primary device handle.
        if (usage_page != 0xFF68 && usage_page != 0xFF60) || usage != 97 {
            continue;
        }

        let path = d.path().to_string_lossy().into_owned();
        if !seen.insert(path.clone()) {
            continue;
        }
        out.push(DeviceSummary {
            path,
            vendor_id: vid,
            product_id: pid,
            product: d.product_string().map(str::to_owned),
            manufacturer: d.manufacturer_string().map(str::to_owned),
            model_name: model,
        });
    }
    Ok(out)
}

/// Read device info from a keyboard by HID path, via our Rust protocol port.
#[tauri::command]
pub fn get_device_info(
    state: tauri::State<AppState>,
    path: String,
) -> Result<protocol::DeviceInfo, String> {
    eprintln!("[RUST LOG] get_device_info called for path={:?}", path);
    let api = state.api.lock().map_err(|e| {
        let err = e.to_string();
        eprintln!("[RUST LOG] get_device_info: lock failed: {}", err);
        err
    })?;
    let cpath = std::ffi::CString::new(path).map_err(|e| {
        let err = e.to_string();
        eprintln!("[RUST LOG] get_device_info: CString conversion failed: {}", err);
        err
    })?;
    let device = api.open_path(&cpath).map_err(|e| {
        let err = format!("open failed: {e}");
        eprintln!("[RUST LOG] get_device_info: {}", err);
        err
    })?;
    match protocol::get_device_info(&device) {
        Ok(info) => {
            eprintln!("[RUST LOG] get_device_info success: vendor_id={:04x}, product_id={:04x}, frame_version={}", info.vendor_id, info.product_id, info.frame_version);
            Ok(info)
        }
        Err(err) => {
            eprintln!("[RUST LOG] get_device_info: protocol error: {}", err);
            Err(err)
        }
    }
}

/// Open a new native settings window for the specific keyboard.
#[tauri::command]
pub async fn open_settings_window(
    app: tauri::AppHandle,
    path: String,
    model: String,
) -> Result<(), String> {
    eprintln!("[RUST LOG] open_settings_window called: path={:?}, model={:?}", path, model);
    let hex_path: String = path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_model: String = model.bytes().map(|b| format!("{:02x}", b)).collect();
    let label = format!("settings-{}", hex_path);
    eprintln!("[RUST LOG] open_settings_window: generated label={:?}", label);

    if let Some(w) = app.get_webview_window(&label) {
        eprintln!("[RUST LOG] open_settings_window: window already exists, setting focus");
        w.set_focus().map_err(|e| {
            let err = e.to_string();
            eprintln!("[RUST LOG] open_settings_window: set_focus failed: {}", err);
            err
        })?;
        return Ok(());
    }

    let url_str = format!("index.html?window=settings&path={}&model={}", hex_path, hex_model);
    eprintln!("[RUST LOG] open_settings_window: loading URL={:?}", url_str);
    let url = tauri::WebviewUrl::App(url_str.into());

    let _window = tauri::WebviewWindowBuilder::new(&app, &label, url)
        .title(format!("Настройки: {}", model))
        .inner_size(950.0, 680.0)
        .resizable(true)
        .build()
        .map_err(|e| {
            let err = e.to_string();
            eprintln!("[RUST LOG] open_settings_window: window build failed: {}", err);
            err
        })?;

    #[cfg(debug_assertions)]
    {
        eprintln!("[RUST LOG] open_settings_window: opening devtools");
        let _ = _window.open_devtools();
    }

    eprintln!("[RUST LOG] open_settings_window success: window created and opened");
    Ok(())
}

#[tauri::command]
pub fn get_game_mode(
    state: tauri::State<AppState>,
    path: String,
    frame_version: u8,
) -> Result<protocol::GameMode, String> {
    println!("[RUST LOG] get_game_mode called for path={:?}", path);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_GAME_MODE, 56, timeout)?;
    let parsed = protocol::parse_game_mode(&payload);
    println!("[RUST LOG] get_game_mode success: game_mode={}", parsed.game_mode);
    Ok(parsed)
}

#[tauri::command]
pub fn set_game_mode(
    state: tauri::State<AppState>,
    path: String,
    config: protocol::GameMode,
    frame_version: u8,
) -> Result<(), String> {
    println!("[RUST LOG] set_game_mode called: path={:?}, game_mode={}", path, config.game_mode);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let data = protocol::encode_game_mode(&config);
    protocol::write_data(&device, protocol::cmd::SET_GAME_MODE, &data, timeout)?;
    println!("[RUST LOG] set_game_mode success");
    Ok(())
}

#[tauri::command]
pub fn get_led_effect(
    state: tauri::State<AppState>,
    path: String,
    frame_version: u8,
) -> Result<protocol::LedEffect, String> {
    println!("[RUST LOG] get_led_effect called: path={:?}", path);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_LED_EFFECT, 16, timeout)?;
    let parsed = protocol::parse_led_effect(&payload);
    println!(
        "[RUST LOG] get_led_effect success: mode={}, red={}, green={}, blue={}, driver_setting={}, color_mode={}, brightness={}, speed={}, direction={}, effect_mode_type={}",
        parsed.mode, parsed.red, parsed.green, parsed.blue, parsed.driver_setting, parsed.color_mode, parsed.brightness, parsed.speed, parsed.direction, parsed.effect_mode_type
    );
    Ok(parsed)
}

#[tauri::command]
pub fn set_led_effect(
    state: tauri::State<AppState>,
    path: String,
    effect: protocol::LedEffect,
) -> Result<(), String> {
    println!(
        "[RUST LOG] set_led_effect called: path={:?}, mode={}, red={}, green={}, blue={}, driver_setting={}, color_mode={}, brightness={}, speed={}, direction={}, effect_mode_type={}",
        path, effect.mode, effect.red, effect.green, effect.blue, effect.driver_setting, effect.color_mode, effect.brightness, effect.speed, effect.direction, effect.effect_mode_type
    );
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let data = protocol::encode_led_effect(&effect);
    protocol::write_data(&device, protocol::cmd::SET_LED_EFFECT, &data, 500)?;
    println!("[RUST LOG] set_led_effect success");
    Ok(())
}

#[tauri::command]
pub fn get_magnetic_rt(
    state: tauri::State<AppState>,
    path: String,
    rt_precision: u8,
    frame_version: u8,
) -> Result<Vec<protocol::MagneticAxisRT>, String> {
    println!("[RUST LOG] get_magnetic_rt called: path={:?}, rt_precision={}", path, rt_precision);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_MAGNETIC_AXIS_RT, 1024, timeout)?;
    let parsed = protocol::parse_magnetic_rt(&payload, rt_precision);
    println!("[RUST LOG] get_magnetic_rt success: axes_count={}", parsed.len());
    Ok(parsed)
}

#[tauri::command]
pub fn set_magnetic_rt(
    state: tauri::State<AppState>,
    path: String,
    rt_precision: u8,
    data: Vec<protocol::MagneticAxisRT>,
    frame_version: u8,
) -> Result<(), String> {
    println!("[RUST LOG] set_magnetic_rt called: path={:?}, rt_precision={}, data_len={}", path, rt_precision, data.len());
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let encoded = protocol::encode_magnetic_rt(&data, rt_precision);
    protocol::write_data(&device, protocol::cmd::SET_MAGNETIC_AXIS_RT, &encoded, timeout)?;
    println!("[RUST LOG] set_magnetic_rt success");
    Ok(())
}

#[tauri::command]
pub fn factory_reset(
    state: tauri::State<AppState>,
    path: String,
    reset_type: u8,
) -> Result<(), String> {
    println!("[RUST LOG] factory_reset called: path={:?}, reset_type={}", path, reset_type);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let packet = protocol::build_packet(protocol::cmd::SET_FACTORY_RESET, reset_type, 0, None, true);
    
    let mut out = [0u8; protocol::REPORT_SIZE + 1];
    out[0] = protocol::REPORT_ID;
    out[1..].copy_from_slice(&packet);
    device.write(&out).map_err(|e| format!("write failed: {e}"))?;
    
    std::thread::sleep(std::time::Duration::from_millis(100));
    println!("[RUST LOG] factory_reset success");
    Ok(())
}

#[tauri::command]
pub fn get_key_data(
    state: tauri::State<AppState>,
    path: String,
    frame_version: u8,
) -> Result<Vec<protocol::RawKeyEntry>, String> {
    println!("[RUST LOG] get_key_data called: path={:?}, frame_version={}", path, frame_version);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_KEY, 512, timeout)?;
    let parsed = protocol::parse_key_data(&payload);
    println!("[RUST LOG] get_key_data success: keys_count={}", parsed.len());
    Ok(parsed)
}

#[tauri::command]
pub fn set_key_data(
    state: tauri::State<AppState>,
    path: String,
    data: Vec<protocol::RawKeyEntry>,
    frame_version: u8,
) -> Result<(), String> {
    println!("[RUST LOG] set_key_data called: path={:?}, data_len={}, frame_version={}", path, data.len(), frame_version);
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let encoded = protocol::encode_key_data(&data);
    protocol::write_data(&device, protocol::cmd::SET_KEY, &encoded, timeout)?;
    println!("[RUST LOG] set_key_data success");
    Ok(())
}

#[tauri::command]
pub fn set_music_data(
    state: tauri::State<AppState>,
    path: String,
    music_type: u8,
    spectrum: Vec<u8>,
) -> Result<(), String> {
    println!("[RUST LOG] set_music_data called: path={:?}, music_type={}, spectrum_len={}", path, music_type, spectrum.len());
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    protocol::send_music_data(&device, music_type, &spectrum)?;
    println!("[RUST LOG] set_music_data success");
    Ok(())
}

#[tauri::command]
pub fn frontend_log(level: String, message: String) {
    eprintln!("[FRONTEND LOG - {}] {}", level, message);
}


