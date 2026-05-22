//! Tauri commands — the IPC surface the Vue frontend calls via invoke().

use crate::{models, protocol};
use hidapi::HidApi;
use std::collections::HashSet;
use std::sync::Mutex;

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
        if usage_page != 0 && usage_page != protocol::USAGE_PAGE {
            continue;
        }
        if usage_page == 0 && model.is_none() {
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
