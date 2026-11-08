use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::{DisplayConfig, ScreenPower};
use crate::platform::{self, DisplayServer};

const BACKLIGHT_ROOT: &str = "/sys/class/backlight";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Backlight {
    pub name: String,

    pub current: u32,

    pub max: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayState {
    pub screen_power_supported: bool,

    pub screen_power_mechanism: &'static str,

    pub brightness_supported: bool,

    pub backlights: Vec<Backlight>,

    pub brightness: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenPowerMethod {
    Wlopm,

    Xset,

    Command,

    None,
}

impl ScreenPowerMethod {
    pub fn name(self) -> &'static str {
        match self {
            Self::Wlopm => "wlopm",
            Self::Xset => "xset",
            Self::Command => "custom command",
            Self::None => "unsupported",
        }
    }
}

pub fn screen_power_method(config: &DisplayConfig) -> ScreenPowerMethod {
    match config.screen_power {
        ScreenPower::Disabled => ScreenPowerMethod::None,
        ScreenPower::Wlopm => {
            if platform::has_program("wlopm") {
                ScreenPowerMethod::Wlopm
            } else {
                ScreenPowerMethod::None
            }
        }
        ScreenPower::Xset => {
            if platform::has_program("xset") {
                ScreenPowerMethod::Xset
            } else {
                ScreenPowerMethod::None
            }
        }
        ScreenPower::Auto => match platform::DisplayServer::detect() {
            DisplayServer::Wayland => {
                if platform::has_program("wlopm") {
                    ScreenPowerMethod::Wlopm
                } else if config.screen_off_command.is_some() {
                    ScreenPowerMethod::Command
                } else {
                    ScreenPowerMethod::None
                }
            }
            DisplayServer::X11 => {
                if platform::has_program("xset") {
                    ScreenPowerMethod::Xset
                } else if config.screen_off_command.is_some() {
                    ScreenPowerMethod::Command
                } else {
                    ScreenPowerMethod::None
                }
            }
            DisplayServer::None => {
                if config.screen_off_command.is_some() {
                    ScreenPowerMethod::Command
                } else {
                    ScreenPowerMethod::None
                }
            }
        },
    }
}

pub fn backlights() -> Vec<Backlight> {
    let Ok(entries) = fs::read_dir(BACKLIGHT_ROOT) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let Some(current) = read_number(&path.join("actual_brightness"))
            .or_else(|| read_number(&path.join("brightness")))
        else {
            continue;
        };
        let max = read_number(&path.join("max_brightness"))
            .unwrap_or(100)
            .max(1);

        found.push(Backlight { name, current, max });
    }

    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn read_number(path: &PathBuf) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    text.trim().parse().ok()
}

fn write_number(path: &PathBuf, value: u32) -> std::io::Result<()> {
    fs::write(path, value.to_string())
}

fn primary_backlight() -> Option<(PathBuf, Backlight)> {
    let devices = backlights();
    let device = devices.first()?.clone();
    let path = PathBuf::from(BACKLIGHT_ROOT)
        .join(&device.name)
        .join("brightness");
    Some((path, device))
}

pub fn brightness() -> Option<u32> {
    let (_, device) = primary_backlight()?;
    Some((device.current.saturating_mul(100) / device.max).min(100))
}

pub fn set_brightness(config: &DisplayConfig, percent: u32) -> Result<(), DisplayError> {
    let percent = percent.min(100);

    if let Some((path, device)) = primary_backlight() {
        let value = device.max.saturating_mul(percent) / 100;
        write_number(&path, value).map_err(|err| DisplayError::Io {
            path: path.to_string_lossy().to_string(),
            message: err.to_string(),
        })?;
        return Ok(());
    }

    if platform::has_program("ddcutil") {
        let value = percent.min(100);
        let status = Command::new("ddcutil")
            .args(["set", "LUMINANCE", &value.to_string()])
            .status()
            .map_err(|err| DisplayError::Command {
                message: err.to_string(),
            })?;
        if status.success() {
            return Ok(());
        }
    }

    if let Some(command) = &config.brightness_command {
        return run_configured_command(command, &percent.to_string());
    }

    Err(DisplayError::Unsupported)
}

pub fn screen_off(config: &DisplayConfig) -> Result<(), DisplayError> {
    match screen_power_method(config) {
        ScreenPowerMethod::Wlopm => {
            let status =
                Command::new("wlopm")
                    .arg("off")
                    .status()
                    .map_err(|err| DisplayError::Command {
                        message: err.to_string(),
                    })?;
            if status.success() {
                Ok(())
            } else {
                Err(DisplayError::Command {
                    message: "wlopm reported a failure".to_string(),
                })
            }
        }
        ScreenPowerMethod::Xset => {
            let status = Command::new("xset")
                .arg("dpms")
                .arg("force")
                .arg("off")
                .status()
                .map_err(|err| DisplayError::Command {
                    message: err.to_string(),
                })?;
            if status.success() {
                Ok(())
            } else {
                Err(DisplayError::Command {
                    message: "xset reported a failure".to_string(),
                })
            }
        }
        ScreenPowerMethod::Command => {
            let command = config
                .screen_off_command
                .as_ref()
                .ok_or(DisplayError::Unsupported)?;
            run_configured_command(command, "")
        }
        ScreenPowerMethod::None => Err(DisplayError::Unsupported),
    }
}

pub fn screen_on(config: &DisplayConfig) -> Result<(), DisplayError> {
    match screen_power_method(config) {
        ScreenPowerMethod::Wlopm => {
            let status =
                Command::new("wlopm")
                    .arg("on")
                    .status()
                    .map_err(|err| DisplayError::Command {
                        message: err.to_string(),
                    })?;
            if status.success() {
                Ok(())
            } else {
                Err(DisplayError::Command {
                    message: "wlopm reported a failure".to_string(),
                })
            }
        }
        ScreenPowerMethod::Xset => {
            let status = Command::new("xset")
                .arg("dpms")
                .arg("force")
                .arg("on")
                .status()
                .map_err(|err| DisplayError::Command {
                    message: err.to_string(),
                })?;
            if status.success() {
                Ok(())
            } else {
                Err(DisplayError::Command {
                    message: "xset reported a failure".to_string(),
                })
            }
        }
        ScreenPowerMethod::Command => {
            let command = config
                .screen_on_command
                .as_ref()
                .ok_or(DisplayError::Unsupported)?;
            run_configured_command(command, "")
        }
        ScreenPowerMethod::None => Err(DisplayError::Unsupported),
    }
}

fn run_configured_command(command: &str, value: &str) -> Result<(), DisplayError> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Err(DisplayError::Command {
            message: "the command is empty".to_string(),
        });
    };

    let args: Vec<String> = parts.map(|arg| arg.replace('%', value)).collect();

    let status =
        Command::new(program)
            .args(&args)
            .status()
            .map_err(|err| DisplayError::Command {
                message: err.to_string(),
            })?;

    if status.success() {
        Ok(())
    } else {
        Err(DisplayError::Command {
            message: format!("{program} reported a failure"),
        })
    }
}

pub fn state(config: &DisplayConfig) -> DisplayState {
    let method = screen_power_method(config);
    let backlights = backlights();

    DisplayState {
        screen_power_supported: method != ScreenPowerMethod::None,
        screen_power_mechanism: method.name(),
        brightness_supported: !backlights.is_empty()
            || platform::has_program("ddcutil")
            || config.brightness_command.is_some(),
        brightness: brightness(),
        backlights,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("this device does not support that operation")]
    Unsupported,

    #[error("the command failed: {message}")]
    Command { message: String },

    #[error("cannot write to {path}: {message}")]
    Io { path: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disabled_screen_power_is_unsupported() {
        let config = DisplayConfig {
            screen_power: ScreenPower::Disabled,
            ..Default::default()
        };
        assert_eq!(screen_power_method(&config), ScreenPowerMethod::None);
    }

    #[test]
    fn an_explicit_backend_is_used_when_it_is_present() {
        let config = DisplayConfig {
            screen_power: ScreenPower::Wlopm,
            ..Default::default()
        };
        let method = screen_power_method(&config);
        if platform::has_program("wlopm") {
            assert_eq!(method, ScreenPowerMethod::Wlopm);
        } else {
            assert_eq!(method, ScreenPowerMethod::None);
        }
    }

    #[test]
    fn a_configured_command_is_used_when_nothing_else_is() {
        let config = DisplayConfig {
            screen_power: ScreenPower::Auto,
            screen_off_command: Some("/usr/bin/true".to_string()),
            ..Default::default()
        };
        let method = screen_power_method(&config);
        if method == ScreenPowerMethod::None {
            assert_eq!(method, ScreenPowerMethod::Command);
        }
    }

    #[test]
    fn a_configured_command_runs_without_a_shell() {
        let result = run_configured_command("/bin/true", "50");
        assert!(result.is_ok());

        let result = run_configured_command("/bin/echo hello", "50");
        assert!(result.is_ok());
    }

    #[test]
    fn a_missing_command_is_an_error_rather_than_a_panic() {
        let result = run_configured_command("", "50");
        assert!(matches!(result, Err(DisplayError::Command { .. })));
    }

    #[test]
    fn a_command_that_does_not_exist_is_an_error() {
        let result = run_configured_command("/nonexistent/program", "");
        assert!(matches!(result, Err(DisplayError::Command { .. })));
    }

    #[test]
    fn the_percentage_is_substituted_into_the_command() {
        let result = run_configured_command("/bin/true %", "42");
        assert!(result.is_ok());
    }
}
