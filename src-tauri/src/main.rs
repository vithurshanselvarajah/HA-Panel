#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = ha_panel_lib::run() {
        eprintln!("ha-panel: {err}");
        std::process::exit(1);
    }
}
