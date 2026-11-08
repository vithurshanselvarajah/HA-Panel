use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    Wayland,

    X11,

    None,
}

impl DisplayServer {
    pub fn detect() -> Self {
        if env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
            Self::Wayland
        } else if env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()) {
            Self::X11
        } else {
            Self::None
        }
    }

    pub fn is_wayland(self) -> bool {
        self == Self::Wayland
    }

    pub fn is_x11(self) -> bool {
        self == Self::X11
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlatformInfo {
    pub display_server: &'static str,

    pub has_desktop: bool,

    pub hostname: String,

    pub version: &'static str,
}

pub fn has_desktop() -> bool {
    for name in [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ] {
        if let Ok(value) = env::var(name) {
            if !value.trim().is_empty() {
                return true;
            }
        }
    }
    false
}

pub fn hostname() -> String {
    if let Ok(name) = env::var("HOSTNAME") {
        if !name.trim().is_empty() {
            return name;
        }
    }

    if let Ok(name) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    "unknown".to_string()
}

pub fn has_program(name: &str) -> bool {
    which(name).is_some()
}

pub fn which(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let path = Path::new(name);
        return path.is_file().then(|| path.to_path_buf());
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn screen_size() -> Option<(u32, u32)> {
    if let Some(size) = wayland_output_size() {
        return Some(size);
    }
    x11_screen_size()
}

fn wayland_output_size() -> Option<(u32, u32)> {
    let output = std::process::Command::new("wlr-randr")
        .arg("--no-header")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let first = text.lines().next()?;
    let dimensions = first.split_whitespace().nth(2)?;
    let (width, height) = dimensions.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn x11_screen_size() -> Option<(u32, u32)> {
    let output = std::process::Command::new("xdpyinfo")
        .arg("--display")
        .arg(":0")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("dimensions:") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let width = parts.next()?.parse().ok()?;
        let height = parts.next()?.parse().ok()?;
        return Some((width, height));
    }
    None
}

pub fn screen_size_or_default() -> (u32, u32) {
    screen_size().unwrap_or((1024, 600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_finds_a_program_on_the_path() {
        assert!(which("sh").is_some());
    }

    #[test]
    fn which_does_not_invent_programs() {
        assert!(which("definitely-not-a-real-program-xyz").is_none());
    }

    #[test]
    fn which_handles_an_explicit_path() {
        assert!(which("/bin/sh").is_some());
        assert!(which("/bin/definitely-not-here").is_none());
    }

    #[test]
    fn the_hostname_is_never_empty() {
        assert!(!hostname().is_empty());
    }

    #[test]
    fn the_screen_size_falls_back_rather_than_failing() {
        let (width, height) = screen_size_or_default();
        assert!(width > 0 && height > 0);
    }

    #[test]
    fn the_display_server_is_one_of_the_three() {
        let server = DisplayServer::detect();
        assert!(matches!(
            server,
            DisplayServer::Wayland | DisplayServer::X11 | DisplayServer::None
        ));
    }
}
