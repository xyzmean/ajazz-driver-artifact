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
        if usage_page != protocol::USAGE_PAGE {
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
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    protocol::get_device_info(&device)
}

/// Open a new native settings window for the specific keyboard.
#[tauri::command]
pub fn open_settings_window(
    app: tauri::AppHandle,
    path: String,
    model: String,
) -> Result<(), String> {
    let hex_path: String = path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_model: String = model.bytes().map(|b| format!("{:02x}", b)).collect();
    let label = format!("settings-{}", hex_path);

    if let Some(w) = app.get_webview_window(&label) {
        w.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let url_str = format!("index.html?window=settings&path={}&model={}", hex_path, hex_model);
    let url = tauri::WebviewUrl::App(std::path::PathBuf::from(url_str));

    let _window = tauri::WebviewWindowBuilder::new(&app, &label, url)
        .title(format!("Настройки: {}", model))
        .inner_size(950.0, 680.0)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_game_mode(
    state: tauri::State<AppState>,
    path: String,
    frame_version: u8,
) -> Result<protocol::GameMode, String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_GAME_MODE, 56, timeout)?;
    Ok(protocol::parse_game_mode(&payload))
}

#[tauri::command]
pub fn set_game_mode(
    state: tauri::State<AppState>,
    path: String,
    config: protocol::GameMode,
    frame_version: u8,
) -> Result<(), String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let data = protocol::encode_game_mode(&config);
    protocol::write_data(&device, protocol::cmd::SET_GAME_MODE, &data, timeout)?;
    Ok(())
}

#[tauri::command]
pub fn get_led_effect(
    state: tauri::State<AppState>,
    path: String,
    frame_version: u8,
) -> Result<protocol::LedEffect, String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_LED_EFFECT, 16, timeout)?;
    Ok(protocol::parse_led_effect(&payload))
}

#[tauri::command]
pub fn set_led_effect(
    state: tauri::State<AppState>,
    path: String,
    effect: protocol::LedEffect,
) -> Result<(), String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let data = protocol::encode_led_effect(&effect);
    protocol::write_data(&device, protocol::cmd::SET_LED_EFFECT, &data, 500)?;
    Ok(())
}

#[tauri::command]
pub fn get_magnetic_rt(
    state: tauri::State<AppState>,
    path: String,
    rt_precision: u8,
    frame_version: u8,
) -> Result<Vec<protocol::MagneticAxisRT>, String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let payload = protocol::read_data(&device, protocol::cmd::GET_MAGNETIC_AXIS_RT, 1024, timeout)?;
    Ok(protocol::parse_magnetic_rt(&payload, rt_precision))
}

#[tauri::command]
pub fn set_magnetic_rt(
    state: tauri::State<AppState>,
    path: String,
    rt_precision: u8,
    data: Vec<protocol::MagneticAxisRT>,
    frame_version: u8,
) -> Result<(), String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let timeout = if frame_version == 1 { 2000 } else { 500 };
    let encoded = protocol::encode_magnetic_rt(&data, rt_precision);
    protocol::write_data(&device, protocol::cmd::SET_MAGNETIC_AXIS_RT, &encoded, timeout)?;
    Ok(())
}

#[tauri::command]
pub fn factory_reset(
    state: tauri::State<AppState>,
    path: String,
    reset_type: u8,
) -> Result<(), String> {
    let api = state.api.lock().map_err(|e| e.to_string())?;
    let cpath = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
    let device = api.open_path(&cpath).map_err(|e| format!("open failed: {e}"))?;
    let packet = protocol::build_packet(protocol::cmd::SET_FACTORY_RESET, reset_type, 0, None, true);
    
    let mut out = [0u8; protocol::REPORT_SIZE + 1];
    out[0] = protocol::REPORT_ID;
    out[1..].copy_from_slice(&packet);
    device.write(&out).map_err(|e| format!("write failed: {e}"))?;
    
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(())
}
