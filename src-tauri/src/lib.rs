pub mod app;
pub mod audio;
pub mod commands;
pub mod config;
pub mod display;
pub mod ha;
pub mod keyboard;
pub mod logging;
pub mod network;
pub mod platform;
pub mod sidecar;

use tauri::{Listener, Manager};

use app::AppState;
use config::ConfigPaths;

pub fn run() -> anyhow::Result<()> {
    let stderr_level = std::env::var("HA_PANEL_LOG")
        .ok()
        .and_then(|value| logging::level_from_name(&value))
        .unwrap_or(log::Level::Warn);
    logging::install(stderr_level);

    let paths = ConfigPaths::system();
    let config = paths.load();
    let secrets = paths.load_secrets();

    if let Err(err) = std::fs::create_dir_all(&paths.state_dir()) {
        log::warn!("cannot create the state directory: {err}");
    }

    log::info!("ha-panel starting");
    log::info!(
        "configuration from {}; display server is {}",
        paths.config_file().display(),
        app::display_server()
    );

    let state = AppState {
        paths,
        config: std::sync::Mutex::new(config.clone()),
        secrets: std::sync::Mutex::new(secrets.clone()),
        sidecar: std::sync::Mutex::new(None),
    };

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state);

    builder = builder.setup(move |app| {
        let handle = app.handle().clone();
        let config = config.clone();

        app::build_windows(&handle, &config, &secrets)?;

        if config.echomuse.enabled {
            let state = handle.state::<AppState>();
            if let Err(err) = commands::start_sidecar_inner(&handle, &state, &config.echomuse) {
                log::error!("cannot start the sidecar: {err}");
            }
        }

        let keyboard_handle = handle.clone();
        app.listen("ha-panel://focus", move |event| {
            let focused = event.payload() == serde_json::Value::Bool(true);
            if focused {
                let _ = app::show_keyboard(&keyboard_handle);
            } else {
                let _ = app::hide_keyboard(&keyboard_handle);
            }
        });

        Ok(())
    });

    builder
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_config,
            commands::get_audio_state,
            commands::get_audio_devices,
            commands::set_volume,
            commands::set_muted,
            commands::set_output_device,
            commands::set_input_device,
            commands::get_display_state,
            commands::set_brightness,
            commands::screen_off,
            commands::screen_on,
            commands::get_wifi_state,
            commands::scan_wifi,
            commands::join_wifi,
            commands::forget_wifi,
            commands::set_wifi_radio,
            commands::open_drawer,
            commands::close_drawer,
            commands::show_keyboard,
            commands::hide_keyboard,
            commands::send_key,
            commands::get_sidecar_state,
            commands::start_sidecar,
            commands::stop_sidecar,
            commands::get_logs,
            commands::clear_logs,
        ])
        .build(tauri::generate_context!())
        .map_err(|err| anyhow::anyhow!("cannot start the application: {err}"))?
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = _app.try_state::<AppState>() {
                    if let Ok(mut guard) = state.sidecar.lock() {
                        if let Some(sidecar) = guard.take() {
                            sidecar.stop();
                        }
                    }
                }
            }
        });

    Ok(())
}
