use serde::Serialize;
use tauri::{Manager, State};

use crate::app::{self, AppState, Status};
use crate::audio;
use crate::config::{Config, EchoMuseConfig};
use crate::display;
use crate::logging;
use crate::network;

#[derive(Debug, Clone, Serialize)]
pub struct DrawerState {
    pub config: Config,

    pub status: Status,

    pub has_token: bool,
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> Result<DrawerState, String> {
    let config = state.config();
    let secrets = state.secrets.lock().map_err(|_| "the secrets are locked")?;

    let sidecar_running = state
        .sidecar
        .lock()
        .map(|sidecar| sidecar.as_ref().is_some_and(|s| s.is_running()))
        .unwrap_or(false);

    Ok(DrawerState {
        status: Status {
            version: env!("CARGO_PKG_VERSION"),
            hostname: crate::platform::hostname(),
            display_server: app::display_server(),
            has_desktop: crate::platform::has_desktop(),
            sidecar_running,
            log_level: logging::level_name(),
            log_records: logging::ring().len(),
        },
        config,
        has_token: secrets.has_ha_token(),
    })
}

#[tauri::command]
pub fn save_config(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    let old = state.config();
    state.set_config(config.clone())?;

    if old.audio.volume != config.audio.volume {
        let backend = audio::detect(config.audio.backend);
        if let Err(err) = audio::set_volume(backend, config.audio.volume) {
            log::warn!("cannot apply the volume: {err}");
        }
    }

    if old.audio.muted != config.audio.muted {
        let backend = audio::detect(config.audio.backend);
        if let Err(err) = audio::set_muted(backend, config.audio.muted) {
            log::warn!("cannot apply the mute state: {err}");
        }
    }

    if let Some(brightness) = config.display.brightness {
        if old.display.brightness != Some(brightness) {
            if let Err(err) = display::set_brightness(&config.display, brightness) {
                log::warn!("cannot apply the brightness: {err}");
            }
        }
    }

    if old.echomuse != config.echomuse {
        restart_sidecar(&app_handle, &state, &config.echomuse)?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_audio_state(config: Config) -> serde_json::Value {
    let backend = audio::detect(config.audio.backend);
    serde_json::to_value(audio::state(backend)).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn set_volume(config: Config, volume: u32) -> Result<(), String> {
    let backend = audio::detect(config.audio.backend);
    audio::set_volume(backend, volume).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_muted(config: Config, muted: bool) -> Result<(), String> {
    let backend = audio::detect(config.audio.backend);
    audio::set_muted(backend, muted).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_audio_devices(config: Config) -> serde_json::Value {
    let backend = audio::detect(config.audio.backend);
    let state = audio::state(backend);
    serde_json::to_value(state).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn set_output_device(config: Config, device: String) -> Result<(), String> {
    let backend = audio::detect(config.audio.backend);
    audio::set_output_device(backend, &device).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_input_device(config: Config, device: String) -> Result<(), String> {
    let backend = audio::detect(config.audio.backend);
    audio::set_input_device(backend, &device).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_display_state(config: Config) -> serde_json::Value {
    serde_json::to_value(display::state(&config.display)).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn set_brightness(config: Config, brightness: u32) -> Result<(), String> {
    display::set_brightness(&config.display, brightness).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn screen_off(config: Config) -> Result<(), String> {
    display::screen_off(&config.display).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn screen_on(config: Config) -> Result<(), String> {
    display::screen_on(&config.display).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_wifi_state() -> serde_json::Value {
    serde_json::to_value(network::state()).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn scan_wifi() -> serde_json::Value {
    serde_json::to_value(network::scan()).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub fn join_wifi(ssid: String, password: Option<String>) -> Result<(), String> {
    network::join(&ssid, password.as_deref()).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn forget_wifi(ssid: String) -> Result<(), String> {
    network::forget(&ssid).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_wifi_radio(on: bool) -> Result<(), String> {
    network::set_radio(on).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn open_drawer(app_handle: tauri::AppHandle) -> Result<(), String> {
    app::open_drawer(&app_handle)
}

#[tauri::command]
pub fn close_drawer(app_handle: tauri::AppHandle) -> Result<(), String> {
    app::close_drawer(&app_handle)
}

#[tauri::command]
pub fn show_keyboard(app_handle: tauri::AppHandle) -> Result<(), String> {
    app::show_keyboard(&app_handle)
}

#[tauri::command]
pub fn hide_keyboard(app_handle: tauri::AppHandle) -> Result<(), String> {
    app::hide_keyboard(&app_handle)
}

#[tauri::command]
pub fn send_key(app_handle: tauri::AppHandle, text: String, backspace: bool) -> Result<(), String> {
    let home = app_handle
        .get_webview_window(app::HOME_WINDOW)
        .ok_or("the Home Assistant window is missing")?;

    let script = crate::keyboard::type_script(&text, backspace);
    home.eval(&script).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_sidecar_state(state: State<'_, AppState>) -> serde_json::Value {
    let guard = match state.sidecar.lock() {
        Ok(guard) => guard,
        Err(_) => return serde_json::Value::Null,
    };

    match guard.as_ref() {
        Some(sidecar) => sidecar.state().unwrap_or(serde_json::Value::Null),
        None => serde_json::Value::Null,
    }
}

#[tauri::command]
pub fn get_logs(min_level: Option<String>) -> String {
    let level = match min_level.as_deref() {
        Some("error") => log::Level::Error,
        Some("warn") => log::Level::Warn,
        Some("info") => log::Level::Info,
        Some("debug") => log::Level::Debug,
        Some("trace") => log::Level::Trace,
        _ => log::Level::Info,
    };

    let mut text = String::new();
    for entry in logging::ring().entries_at_or_above(level) {
        text.push_str(&format!(
            "[{:<5}] {}: {}\n",
            entry.level, entry.target, entry.message
        ));
    }
    text
}

#[tauri::command]
pub fn clear_logs() {
    logging::ring().clear();
}

#[tauri::command]
pub fn start_sidecar(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = state.config();
    start_sidecar_inner(&app_handle, &state, &config.echomuse)
}

#[tauri::command]
pub fn stop_sidecar(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.sidecar.lock().map_err(|_| "the sidecar is locked")?;
    if let Some(sidecar) = guard.take() {
        sidecar.stop();
    }
    Ok(())
}

pub fn start_sidecar_inner(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config: &EchoMuseConfig,
) -> Result<(), String> {
    if !config.enabled {
        return Err("the EchoMuse integration is not enabled".to_string());
    }

    let binary = sidecar_binary();
    let config_json = serde_json::to_string(config).map_err(|err| err.to_string())?;

    let mut guard = state.sidecar.lock().map_err(|_| "the sidecar is locked")?;
    if let Some(existing) = guard.as_ref() {
        if existing.is_running() {
            return Ok(());
        }
    }
    *guard = None;

    let sidecar = crate::sidecar::Sidecar::start(
        &binary,
        std::path::Path::new(&config.socket_path),
        &config_json,
    )
    .map_err(|err| err.to_string())?;

    *guard = Some(sidecar);

    let socket_path = config.socket_path.clone();
    let state_for_socket = app_handle.clone();
    if let Err(err) =
        crate::sidecar::serve_socket(std::path::Path::new(&socket_path), move |line| {
            let _ = &state_for_socket;
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "result": {
                    "note": "the sidecar serves this socket; the app reads its state directly"
                },
                "echo": line
            })
            .to_string()
        })
    {
        log::warn!("cannot serve the sidecar socket: {err}");
    }

    Ok(())
}

fn restart_sidecar(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config: &EchoMuseConfig,
) -> Result<(), String> {
    {
        let mut guard = state.sidecar.lock().map_err(|_| "the sidecar is locked")?;
        if let Some(sidecar) = guard.take() {
            sidecar.stop();
        }
    }

    if config.enabled {
        start_sidecar_inner(app_handle, state, config)?;
    }
    Ok(())
}

fn sidecar_binary() -> std::path::PathBuf {
    std::path::PathBuf::from("ha-panel-sidecar")
}
