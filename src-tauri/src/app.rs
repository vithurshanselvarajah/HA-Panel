use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::{Config, ConfigPaths, Secrets};
use crate::platform::{self, DisplayServer};
use crate::sidecar::Sidecar;

pub const HOME_WINDOW: &str = "home";

pub const DRAWER_WINDOW: &str = "drawer";

pub const KEYBOARD_WINDOW: &str = "keyboard";

pub const HANDLE_WINDOW: &str = "handle";

pub struct AppState {
    pub paths: ConfigPaths,

    pub config: Mutex<Config>,

    pub secrets: Mutex<Secrets>,

    pub sidecar: Mutex<Option<Sidecar>>,
}

impl AppState {
    pub fn config(&self) -> Config {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn set_config(&self, config: Config) -> Result<(), String> {
        self.paths
            .save(&config)
            .map_err(|err| format!("cannot write the configuration: {err}"))?;
        *self
            .config
            .lock()
            .map_err(|_| "the configuration is locked")? = config;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub version: &'static str,

    pub hostname: String,

    pub display_server: &'static str,

    pub has_desktop: bool,

    pub sidecar_running: bool,

    pub log_level: &'static str,

    pub log_records: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardState {
    pub config: Config,

    pub status: Status,

    pub has_token: bool,
}

pub fn build_windows(
    app: &tauri::AppHandle,
    config: &Config,
    secrets: &Secrets,
) -> anyhow::Result<()> {
    let (screen_width, screen_height) = platform::screen_size_or_default();

    if config.kiosk.show_handle {
        let handle_width = config.kiosk.handle_width.min(screen_width.max(1));
        WebviewWindowBuilder::new(app, HANDLE_WINDOW, WebviewUrl::App("index.html".into()))
            .title("ha-panel handle")
            .inner_size(handle_width as f64, screen_height as f64)
            .position((screen_width.saturating_sub(handle_width)) as f64, 0.0)
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .visible(true)
            .build()?;
    }

    let drawer_width = config.kiosk.drawer_width.min(screen_width.max(1));
    WebviewWindowBuilder::new(app, DRAWER_WINDOW, WebviewUrl::App("index.html".into()))
        .title("ha-panel drawer")
        .inner_size(drawer_width as f64, screen_height as f64)
        .position((screen_width.saturating_sub(drawer_width)) as f64, 0.0)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .build()?;

    if config.kiosk.on_screen_keyboard {
        WebviewWindowBuilder::new(
            app,
            KEYBOARD_WINDOW,
            WebviewUrl::App("keyboard.html".into()),
        )
        .title("ha-panel keyboard")
        .inner_size(screen_width as f64, (screen_height as f64 * 0.4).min(480.0))
        .position(0.0, screen_height as f64 * 0.6)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .build()?;
    }

    let url = crate::config::dashboard_url(config);
    let mut home = WebviewWindowBuilder::new(app, HOME_WINDOW, WebviewUrl::External(url.parse()?))
        .title("Home Assistant")
        .inner_size(screen_width as f64, screen_height as f64)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .shadow(false);

    if let Some(token) = secrets.ha_long_lived_token.as_deref() {
        home = home.initialization_script(crate::ha::token_script(token));
    }
    home = home.initialization_script(crate::ha::kiosk_style_script());

    if config.kiosk.on_screen_keyboard {
        home = home.initialization_script(crate::keyboard::watch_focus_script());
    }

    let home = home.build()?;

    if config.ha.fullscreen {
        let _ = home.set_fullscreen(true);
    }

    Ok(())
}

pub fn open_drawer(app: &tauri::AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window(DRAWER_WINDOW)
        .ok_or("the drawer window is missing")?;
    drawer.show().map_err(|err| err.to_string())?;
    drawer.set_focus().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn close_drawer(app: &tauri::AppHandle) -> Result<(), String> {
    let drawer = app
        .get_webview_window(DRAWER_WINDOW)
        .ok_or("the drawer window is missing")?;
    drawer.hide().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn show_keyboard(app: &tauri::AppHandle) -> Result<(), String> {
    let keyboard = app
        .get_webview_window(KEYBOARD_WINDOW)
        .ok_or("the keyboard window is missing")?;
    keyboard.show().map_err(|err| err.to_string())?;
    Ok(())
}

pub fn hide_keyboard(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(keyboard) = app.get_webview_window(KEYBOARD_WINDOW) {
        let _ = keyboard.hide();
    }
    Ok(())
}

pub fn display_server() -> &'static str {
    match DisplayServer::detect() {
        DisplayServer::Wayland => "wayland",
        DisplayServer::X11 => "x11",
        DisplayServer::None => "none",
    }
}
