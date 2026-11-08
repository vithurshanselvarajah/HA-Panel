use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const CONFIG_DIR: &str = "/etc/ha-panel";

pub const CONFIG_FILE: &str = "config.json";

pub const SECRETS_FILE: &str = "secrets.json";

pub const CONFIG_MODE: u32 = 0o660;

pub const SECRETS_MODE: u32 = 0o600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBackend {
    Auto,

    Pulse,

    PipeWire,

    Alsa,
}

impl AudioBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Pulse => "PulseAudio",
            Self::PipeWire => "PipeWire",
            Self::Alsa => "ALSA",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenPower {
    Auto,

    Wlopm,

    Xset,

    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HaConfig {
    pub url: String,

    pub dashboard_path: String,

    pub fullscreen: bool,

    pub reload_on_reconnect: bool,
}

impl Default for HaConfig {
    fn default() -> Self {
        Self {
            url: "http://homeassistant.local:8123".to_string(),
            dashboard_path: "/lovelace".to_string(),
            fullscreen: true,
            reload_on_reconnect: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KioskConfig {
    pub handle_width: u32,

    pub drawer_width: u32,

    pub show_handle: bool,

    pub on_screen_keyboard: bool,

    pub screen_off_timeout_secs: u64,

    pub screen_on_timeout_secs: u64,
}

impl Default for KioskConfig {
    fn default() -> Self {
        Self {
            handle_width: 12,
            drawer_width: 380,
            show_handle: true,
            on_screen_keyboard: true,
            screen_off_timeout_secs: 0,
            screen_on_timeout_secs: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub backend: AudioBackend,

    pub volume: u32,

    pub muted: bool,

    pub output_device: Option<String>,

    pub input_device: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: AudioBackend::Auto,
            volume: 50,
            muted: false,
            output_device: None,
            input_device: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub screen_power: ScreenPower,

    pub brightness: Option<u32>,

    pub brightness_command: Option<String>,

    pub screen_off_command: Option<String>,

    pub screen_on_command: Option<String>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            screen_power: ScreenPower::Auto,
            brightness: None,
            brightness_command: None,
            screen_off_command: None,
            screen_on_command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub wifi_enabled: bool,

    pub preferred_connection: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            wifi_enabled: true,
            preferred_connection: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EchoMuseConfig {
    pub enabled: bool,

    pub controller_url: Option<String>,

    pub controller_tls_port: u16,

    pub device_id: Option<String>,

    pub board: String,

    pub version: String,

    pub credentials_dir: String,

    pub socket_path: String,

    pub input_device: Option<String>,

    pub output_device: Option<String>,
}

impl Default for EchoMuseConfig {
    fn default() -> Self {
        let state_dir = std::env::var_os("HA_PANEL_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/ha-panel"));

        Self {
            enabled: false,
            controller_url: None,
            controller_tls_port: 0,
            device_id: None,
            board: "ha-panel".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            credentials_dir: state_dir.join("echomuse").to_string_lossy().to_string(),
            socket_path: state_dir.join("sidecar.sock").to_string_lossy().to_string(),
            input_device: None,
            output_device: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ha: HaConfig,

    pub kiosk: KioskConfig,

    pub audio: AudioConfig,

    pub display: DisplayConfig,

    pub network: NetworkConfig,

    pub echomuse: EchoMuseConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Secrets {
    pub ha_long_lived_token: Option<String>,
}

impl Secrets {
    pub fn has_ha_token(&self) -> bool {
        self.ha_long_lived_token
            .as_ref()
            .is_some_and(|t| !t.trim().is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub dir: PathBuf,
}

impl ConfigPaths {
    pub fn system() -> Self {
        let dir = std::env::var_os("HA_PANEL_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(CONFIG_DIR));

        Self { dir }
    }

    pub fn under(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn state_dir(&self) -> PathBuf {
        std::env::var_os("HA_PANEL_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/ha-panel"))
    }

    pub fn config_file(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    pub fn secrets_file(&self) -> PathBuf {
        self.dir.join(SECRETS_FILE)
    }

    pub fn load(&self) -> Config {
        match fs::read(self.config_file()) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
                log::warn!("configuration file is not valid JSON ({err}); using defaults");
                Config::default()
            }),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Config::default(),
            Err(err) => {
                log::warn!("cannot read configuration ({err}); using defaults");
                Config::default()
            }
        }
    }

    pub fn load_secrets(&self) -> Secrets {
        match fs::read(self.secrets_file()) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Secrets::default(),
            Err(err) => {
                log::warn!("cannot read secrets ({err}); continuing without them");
                Secrets::default()
            }
        }
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(config)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        write_atomic(&self.config_file(), &bytes, CONFIG_MODE)
    }

    pub fn save_secrets(&self, secrets: &Secrets) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(secrets)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        write_atomic(&self.secrets_file(), &bytes, SECRETS_MODE)
    }
}

fn write_atomic(path: &Path, contents: &[u8], mode: u32) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        )
    })?;

    fs::create_dir_all(dir)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config");
    let tmp = dir.join(format!(".{file_name}.tmp"));

    {
        let mut file = fs::File::create(&tmp)?;
        use std::io::Write;
        file.write_all(contents)?;
        file.sync_all()?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
    }

    match fs::rename(&tmp, path) {
        Ok(()) => {}
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
    }

    if let Ok(dir_handle) = fs::File::open(dir) {
        let _ = dir_handle.sync_all();
    }

    Ok(())
}

pub fn dashboard_url(config: &Config) -> String {
    let base = config.ha.url.trim_end_matches('/');
    let path = config.ha.dashboard_path.trim();
    if path.is_empty() || path == "/" {
        base.to_string()
    } else {
        format!("{base}/{}", path.trim_start_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_url_joins_base_and_path() {
        let mut config = Config::default();
        config.ha.url = "https://ha.example.com/".to_string();
        config.ha.dashboard_path = "/lovelace".to_string();
        assert_eq!(dashboard_url(&config), "https://ha.example.com/lovelace");

        config.ha.url = "http://10.0.0.5:8123".to_string();
        assert_eq!(dashboard_url(&config), "http://10.0.0.5:8123/lovelace");
    }

    #[test]
    fn dashboard_url_tolerates_a_missing_path() {
        let mut config = Config::default();
        config.ha.url = "http://10.0.0.5:8123".to_string();
        config.ha.dashboard_path = "/".to_string();
        assert_eq!(dashboard_url(&config), "http://10.0.0.5:8123");

        config.ha.dashboard_path = String::new();
        assert_eq!(dashboard_url(&config), "http://10.0.0.5:8123");
    }

    #[test]
    fn config_round_trips_through_a_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());

        let mut config = Config::default();
        config.ha.url = "https://ha.example.com".to_string();
        config.audio.volume = 73;

        paths.save(&config).expect("save");
        let loaded = paths.load();
        assert_eq!(loaded.ha.url, "https://ha.example.com");
        assert_eq!(loaded.audio.volume, 73);
    }

    #[test]
    fn a_missing_config_file_yields_the_defaults() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());
        assert_eq!(paths.load().ha.url, HaConfig::default().url);
    }

    #[test]
    fn a_corrupt_config_file_yields_the_defaults() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());
        fs::write(paths.config_file(), b"{ not json").expect("write");
        assert_eq!(paths.load().ha.url, HaConfig::default().url);
    }

    #[test]
    fn unknown_keys_are_ignored_rather_than_fatal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());
        fs::write(
            paths.config_file(),
            br#"{"ha":{"url":"http://x"},"something_new":42}"#,
        )
        .expect("write");
        assert_eq!(paths.load().ha.url, "http://x");
    }

    #[test]
    fn the_secrets_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());
        paths.save_secrets(&Secrets::default()).expect("save");
        let mode = fs::metadata(paths.secrets_file())
            .expect("stat")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, SECRETS_MODE);
    }

    #[test]
    fn the_config_file_is_group_writable() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = ConfigPaths::under(dir.path());
        paths.save(&Config::default()).expect("save");
        let mode = fs::metadata(paths.config_file())
            .expect("stat")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, CONFIG_MODE);
    }
}
