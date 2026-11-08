use serde::Serialize;
use std::process::Command;

use crate::platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WifiNetwork {
    pub ssid: String,

    pub signal: u32,

    pub secured: bool,

    pub connected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WifiState {
    pub available: bool,

    pub radio_present: bool,

    pub radio_on: bool,

    pub interface: Option<String>,

    pub connected_ssid: Option<String>,

    pub networks: Vec<WifiNetwork>,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("network management is not available on this device")]
    Unavailable,

    #[error("the network command failed: {message}")]
    Command { message: String },

    #[error("no network named {0} was found")]
    NotFound(String),
}

pub fn is_available() -> bool {
    platform::has_program("nmcli")
        && Command::new("nmcli")
            .args(["-t", "general", "status"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
}

fn wifi_interface() -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE", "device", "status"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let fields: Vec<String> = line.split(':').map(unescape).collect();
        if fields.len() >= 2 && fields[1] == "wifi" {
            return Some(fields[0].clone());
        }
    }
    None
}

fn connected_ssid() -> Option<String> {
    let output = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "NAME,TYPE,DEVICE",
            "connection",
            "show",
            "--active",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let fields: Vec<String> = line.split(':').map(unescape).collect();
        if fields.len() >= 3 && fields[1] == "802-11-wireless" {
            return Some(fields[0].clone());
        }
    }
    None
}

fn radio_on() -> bool {
    Command::new("nmcli")
        .args(["radio", "wifi"])
        .output()
        .map(|out| {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            text.contains("enabled")
        })
        .unwrap_or(false)
}

pub fn state() -> WifiState {
    let available = is_available();
    let interface = if available { wifi_interface() } else { None };

    let mut wifi = WifiState {
        available,
        radio_present: interface.is_some(),
        radio_on: if available { radio_on() } else { false },
        interface: interface.clone(),
        connected_ssid: if available { connected_ssid() } else { None },
        networks: Vec::new(),
    };

    let Some(interface) = interface else {
        return wifi;
    };

    wifi.networks = list_networks(&interface);
    wifi
}

pub fn scan() -> Vec<WifiNetwork> {
    let Some(interface) = wifi_interface() else {
        return Vec::new();
    };

    let _ = Command::new("nmcli")
        .args(["device", "wifi", "rescan", "ifname", &interface])
        .status();

    list_networks(&interface)
}

fn list_networks(interface: &str) -> Vec<WifiNetwork> {
    let Ok(output) = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY",
            "device",
            "wifi",
            "list",
            "ifname",
            interface,
        ])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let fields = split_escaped(line, ':');
        if fields.len() < 4 {
            continue;
        }

        let in_use = fields[0].trim() == "*";
        let ssid = fields[1].trim().to_string();
        if ssid.is_empty() {
            continue;
        }

        let signal = fields[2].trim().parse().unwrap_or(0);
        let security = fields[3].trim();
        let secured = !security.is_empty() && security != "--";

        networks.push(WifiNetwork {
            ssid,
            signal,
            secured,
            connected: in_use,
        });
    }

    networks
}

pub fn join(ssid: &str, password: Option<&str>) -> Result<(), NetworkError> {
    if !is_available() {
        return Err(NetworkError::Unavailable);
    }

    if connection_exists(ssid)? {
        let status = Command::new("nmcli")
            .args(["connection", "up", ssid])
            .status()
            .map_err(|err| NetworkError::Command {
                message: err.to_string(),
            })?;
        return if status.success() {
            Ok(())
        } else {
            Err(NetworkError::Command {
                message: format!("could not bring up the connection for {ssid}"),
            })
        };
    }

    let password = password.unwrap_or("");
    let status = Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", password])
        .status()
        .map_err(|err| NetworkError::Command {
            message: err.to_string(),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(NetworkError::Command {
            message: format!("could not connect to {ssid}"),
        })
    }
}

fn connection_exists(ssid: &str) -> Result<bool, NetworkError> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "NAME,TYPE", "connection", "show"])
        .output()
        .map_err(|err| NetworkError::Command {
            message: err.to_string(),
        })?;

    if !output.status.success() {
        return Err(NetworkError::Command {
            message: "could not list the connections".to_string(),
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(|line| {
        let fields = split_escaped(line, ':');
        fields.len() >= 2 && fields[0] == ssid && fields[1] == "802-11-wireless"
    }))
}

pub fn forget(ssid: &str) -> Result<(), NetworkError> {
    let status = Command::new("nmcli")
        .args(["connection", "delete", ssid])
        .status()
        .map_err(|err| NetworkError::Command {
            message: err.to_string(),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(NetworkError::NotFound(ssid.to_string()))
    }
}

pub fn set_radio(on: bool) -> Result<(), NetworkError> {
    let status = Command::new("nmcli")
        .args(["radio", "wifi", if on { "on" } else { "off" }])
        .status()
        .map_err(|err| NetworkError::Command {
            message: err.to_string(),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(NetworkError::Command {
            message: "could not change the radio state".to_string(),
        })
    }
}

fn split_escaped(line: &str, separator: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut characters = line.chars();

    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                if let Some(next) = characters.next() {
                    current.push(next);
                }
            }
            c if c == separator => {
                fields.push(std::mem::take(&mut current));
            }
            c => current.push(c),
        }
    }

    fields.push(current);
    fields
}

fn unescape(value: &str) -> String {
    split_escaped(value, ':')
        .first()
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_escaped_colon_does_not_split_a_field() {
        let fields = split_escaped("My\\:Network:802-11-wireless:80:WPA2", ':');
        assert_eq!(fields[0], "My:Network");
        assert_eq!(fields[1], "802-11-wireless");
        assert_eq!(fields[2], "80");
        assert_eq!(fields[3], "WPA2");
    }

    #[test]
    fn an_escaped_backslash_is_literal() {
        let fields = split_escaped("a\\\\b:c", ':');
        assert_eq!(fields[0], "a\\b");
        assert_eq!(fields[1], "c");
    }

    #[test]
    fn a_plain_line_splits_into_its_fields() {
        let fields = split_escaped("VM0470404:802-11-wireless", ':');
        assert_eq!(fields[0], "VM0470404");
        assert_eq!(fields[1], "802-11-wireless");
    }

    #[test]
    fn an_empty_line_produces_one_empty_field() {
        let fields = split_escaped("", ':');
        assert_eq!(fields.len(), 1);
        assert!(fields[0].is_empty());
    }

    #[test]
    fn the_state_does_not_panic_when_network_manager_is_missing() {
        let state = state();
        if !state.available {
            assert!(!state.radio_present);
            assert!(state.networks.is_empty());
        }
    }
}
