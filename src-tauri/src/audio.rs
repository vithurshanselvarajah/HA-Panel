use serde::Serialize;
use std::process::Command;

use crate::config::AudioBackend;
use crate::platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    pub id: String,

    pub name: String,

    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioState {
    pub backend: &'static str,

    pub available: bool,

    pub volume: u32,

    pub muted: bool,

    pub outputs: Vec<Device>,

    pub inputs: Vec<Device>,

    pub selected_output: Option<String>,

    pub selected_input: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    PipeWire,

    Pulse,

    Alsa,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Self::PipeWire => "PipeWire",
            Self::Pulse => "PulseAudio",
            Self::Alsa => "ALSA",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no audio backend is available")]
    NoBackend,

    #[error("{backend} command failed: {message}")]
    Command {
        backend: &'static str,

        message: String,
    },

    #[error("could not read the {backend} output: {message}")]
    Parse {
        backend: &'static str,

        message: String,
    },
}

pub fn detect(preference: AudioBackend) -> Backend {
    let available = |backend: Backend| -> bool {
        match backend {
            Backend::PipeWire => platform::has_program("wpctl"),
            Backend::Pulse => platform::has_program("pactl"),
            Backend::Alsa => platform::has_program("amixer"),
        }
    };

    match preference {
        AudioBackend::PipeWire if available(Backend::PipeWire) => Backend::PipeWire,
        AudioBackend::Pulse if available(Backend::Pulse) => Backend::Pulse,
        AudioBackend::Alsa if available(Backend::Alsa) => Backend::Alsa,
        AudioBackend::PipeWire | AudioBackend::Pulse | AudioBackend::Alsa => {
            log::warn!("the requested audio backend is not available; falling back to another one");
            if available(Backend::PipeWire) {
                Backend::PipeWire
            } else if available(Backend::Pulse) {
                Backend::Pulse
            } else if available(Backend::Alsa) {
                Backend::Alsa
            } else {
                Backend::Alsa
            }
        }
        AudioBackend::Auto => {
            if available(Backend::PipeWire) {
                Backend::PipeWire
            } else if available(Backend::Pulse) {
                Backend::Pulse
            } else if available(Backend::Alsa) {
                Backend::Alsa
            } else {
                Backend::Alsa
            }
        }
    }
}

pub fn is_available(backend: Backend) -> bool {
    match backend {
        Backend::PipeWire => platform::has_program("wpctl"),
        Backend::Pulse => platform::has_program("pactl"),
        Backend::Alsa => platform::has_program("amixer"),
    }
}

pub fn state(backend: Backend) -> AudioState {
    match backend {
        Backend::PipeWire => pipewire_state(),
        Backend::Pulse => pulse_state(),
        Backend::Alsa => alsa_state(),
    }
}

pub fn set_volume(backend: Backend, volume: u32) -> Result<(), AudioError> {
    let volume = volume.min(100);
    match backend {
        Backend::PipeWire => run(
            "PipeWire",
            Command::new("wpctl").args([
                "set-volume",
                "@DEFAULT_AUDIO_SINK@",
                &format!("{volume}%"),
            ]),
        ),
        Backend::Pulse => run(
            "PulseAudio",
            Command::new("pactl").args([
                "set-sink-volume",
                "@DEFAULT_SINK@",
                &format!("{volume}%"),
            ]),
        ),
        Backend::Alsa => run(
            "ALSA",
            Command::new("amixer").args(["-q", "set", "Master", &format!("{volume}%")]),
        ),
    }
}

pub fn set_muted(backend: Backend, muted: bool) -> Result<(), AudioError> {
    let value = if muted { "1" } else { "0" };
    match backend {
        Backend::PipeWire => run(
            "PipeWire",
            Command::new("wpctl").args(["set-mute", "@DEFAULT_AUDIO_SINK@", value]),
        ),
        Backend::Pulse => run(
            "PulseAudio",
            Command::new("pactl").args(["set-sink-mute", "@DEFAULT_SINK@", value]),
        ),
        Backend::Alsa => run(
            "ALSA",
            Command::new("amixer").args([
                "-q",
                "set",
                "Master",
                if muted { "mute" } else { "unmute" },
            ]),
        ),
    }
}

pub fn set_output_device(backend: Backend, device: &str) -> Result<(), AudioError> {
    match backend {
        Backend::PipeWire => run(
            "PipeWire",
            Command::new("wpctl").args(["set-default", device]),
        ),
        Backend::Pulse => run(
            "PulseAudio",
            Command::new("pactl").args(["set-default-sink", device]),
        ),

        Backend::Alsa => Err(AudioError::Parse {
            backend: "ALSA",
            message: "ALSA does not support choosing a playback device".to_string(),
        }),
    }
}

pub fn set_input_device(backend: Backend, device: &str) -> Result<(), AudioError> {
    match backend {
        Backend::PipeWire => run(
            "PipeWire",
            Command::new("wpctl").args(["set-default", device]),
        ),
        Backend::Pulse => run(
            "PulseAudio",
            Command::new("pactl").args(["set-default-source", device]),
        ),
        Backend::Alsa => Err(AudioError::Parse {
            backend: "ALSA",
            message: "ALSA does not support choosing a capture device".to_string(),
        }),
    }
}

fn run(backend: &'static str, command: &mut Command) -> Result<(), AudioError> {
    let output = command.output().map_err(|err| AudioError::Command {
        backend,
        message: err.to_string(),
    })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AudioError::Command { backend, message });
    }

    Ok(())
}

fn capture(backend: &'static str, command: &mut Command) -> Result<String, AudioError> {
    let output = command.output().map_err(|err| AudioError::Command {
        backend,
        message: err.to_string(),
    })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AudioError::Command { backend, message });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn pipewire_state() -> AudioState {
    let mut state = AudioState {
        backend: Backend::PipeWire.name(),
        available: true,
        volume: 0,
        muted: false,
        outputs: Vec::new(),
        inputs: Vec::new(),
        selected_output: None,
        selected_input: None,
    };

    let Ok(text) = capture("PipeWire", Command::new("wpctl").arg("status")) else {
        state.available = false;
        return state;
    };

    let Some(audio) = section_of(&text, "Audio") else {
        state.available = false;
        return state;
    };

    let (outputs, selected_output) = parse_wpctl_section(&audio, "Sinks");
    let (inputs, selected_input) = parse_wpctl_section(&audio, "Sources");
    state.outputs = outputs;
    state.inputs = inputs;
    state.selected_output = selected_output;
    state.selected_input = selected_input;

    let sink = state
        .selected_output
        .clone()
        .or_else(|| state.outputs.first().map(|d| d.id.clone()));

    if let Some(sink) = sink {
        if let Ok(text) = capture(
            "PipeWire",
            Command::new("wpctl").args(["get-volume", &sink]),
        ) {
            state.volume = parse_wpctl_volume(&text).unwrap_or(0);
        }
        if let Ok(text) = capture("PipeWire", Command::new("wpctl").args(["get-mute", &sink])) {
            state.muted = parse_wpctl_mute(&text);
        }
    }

    state
}

fn pulse_state() -> AudioState {
    let mut state = AudioState {
        backend: Backend::Pulse.name(),
        available: true,
        volume: 0,
        muted: false,
        outputs: Vec::new(),
        inputs: Vec::new(),
        selected_output: None,
        selected_input: None,
    };

    let Ok(sinks) = capture(
        "PulseAudio",
        Command::new("pactl").args(["list", "short", "sinks"]),
    ) else {
        state.available = false;
        return state;
    };

    for line in sinks.lines() {
        let mut fields = line.split_whitespace();
        let Some(id) = fields.next() else {
            continue;
        };
        let name = line.split_whitespace().nth(1).unwrap_or(id).to_string();
        state.outputs.push(Device {
            id: id.to_string(),
            name,
            default: false,
        });
    }

    let Ok(sources) = capture(
        "PulseAudio",
        Command::new("pactl").args(["list", "short", "sources"]),
    ) else {
        state.available = false;
        return state;
    };

    for line in sources.lines() {
        let mut fields = line.split_whitespace();
        let Some(id) = fields.next() else {
            continue;
        };
        let name = line.split_whitespace().nth(1).unwrap_or(id).to_string();
        state.inputs.push(Device {
            id: id.to_string(),
            name,
            default: false,
        });
    }

    if let Ok(text) = capture("PulseAudio", Command::new("pactl").args(["list", "sinks"])) {
        state.selected_output = pulse_default(&text, "Sink");
    }
    if let Ok(text) = capture(
        "PulseAudio",
        Command::new("pactl").args(["list", "sources"]),
    ) {
        state.selected_input = pulse_default(&text, "Source");
    }

    if let Some(sink) = state.selected_output.clone() {
        if let Ok(text) = capture(
            "PulseAudio",
            Command::new("pactl").args(["get-sink-volume", &sink]),
        ) {
            state.volume = parse_pactl_volume(&text).unwrap_or(0);
        }
        if let Ok(text) = capture(
            "PulseAudio",
            Command::new("pactl").args(["get-sink-mute", &sink]),
        ) {
            state.muted = text.trim() == "yes";
        }
    }

    state
}

fn alsa_state() -> AudioState {
    let mut state = AudioState {
        backend: Backend::Alsa.name(),
        available: true,
        volume: 0,
        muted: false,
        outputs: Vec::new(),
        inputs: Vec::new(),
        selected_output: None,
        selected_input: None,
    };

    let Ok(text) = capture("ALSA", Command::new("amixer").arg("scontrols")) else {
        state.available = false;
        return state;
    };

    for line in text.lines() {
        let Some(rest) = line.split('\'').nth(1) else {
            continue;
        };
        let name = rest.split('\'').next().unwrap_or(rest);
        state.outputs.push(Device {
            id: name.to_string(),
            name: name.to_string(),
            default: name == "Master",
        });
    }

    state.selected_output = Some("Master".to_string());

    if let Ok(text) = capture("ALSA", Command::new("amixer").arg("get").arg("Master")) {
        state.volume = parse_amixer_volume(&text).unwrap_or(0);
        state.muted = parse_amixer_muted(&text);
    }

    state
}

fn section_of(text: &str, name: &str) -> Option<String> {
    let mut collecting = false;
    let mut section = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let is_header = !trimmed.is_empty()
            && !trimmed.starts_with('├')
            && !trimmed.starts_with('└')
            && !trimmed.starts_with('│')
            && !trimmed.starts_with(' ');

        if is_header {
            if collecting {
                break;
            }
            if trimmed == name {
                collecting = true;
            }
            continue;
        }

        if collecting {
            section.push_str(line);
            section.push('\n');
        }
    }

    if collecting && !section.is_empty() {
        Some(section)
    } else {
        None
    }
}

fn strip_tree(line: &str) -> &str {
    line.trim_start_matches(|c| matches!(c, '│' | '├' | '└' | '─' | ' '))
}

fn parse_wpctl_section(section: &str, heading: &str) -> (Vec<Device>, Option<String>) {
    let mut devices = Vec::new();
    let mut default = None;
    let mut in_section = false;

    for line in section.lines() {
        let trimmed = strip_tree(line);

        if trimmed.ends_with(':') && !trimmed.starts_with('[') {
            in_section = trimmed.trim_end_matches(':') == heading;
            continue;
        }

        if !in_section {
            continue;
        }

        let (marker, rest) = match trimmed.strip_prefix("*") {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };

        let rest = rest.trim_start();
        let Some((number, name)) = rest.split_once(". ") else {
            continue;
        };
        if number.trim().parse::<u32>().is_err() {
            continue;
        }

        let name = name.split(" [vol:").next().unwrap_or(name).trim();

        devices.push(Device {
            id: number.trim().to_string(),
            name: name.to_string(),
            default: marker,
        });

        if marker {
            default = Some(number.trim().to_string());
        }
    }

    (devices, default)
}

fn parse_wpctl_volume(text: &str) -> Option<u32> {
    let value = text.split(':').nth(1)?.trim();
    let value = value.split_whitespace().next()?;
    let fraction: f64 = value.parse().ok()?;
    Some((fraction * 100.0).round().clamp(0.0, 100.0) as u32)
}

fn parse_wpctl_mute(text: &str) -> bool {
    let lowered = text.to_lowercase();
    lowered.contains("[muted]") || lowered.trim_end().ends_with("muted")
}

fn parse_pactl_volume(text: &str) -> Option<u32> {
    let percent = text.split('%').next()?;
    let value = percent.split('/').next_back()?.trim();
    value.parse().ok()
}

fn pulse_default(text: &str, kind: &str) -> Option<String> {
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with(&format!("{kind} #")) {
            in_block = true;
            continue;
        }
        if in_block && line.starts_with('\t') {
            if let Some(name) = line.strip_prefix("\tName: ") {
                return Some(name.trim().to_string());
            }
        }
        if in_block && !line.starts_with('\t') && !line.trim().is_empty() {
            in_block = false;
        }
    }
    None
}

fn parse_amixer_volume(text: &str) -> Option<u32> {
    for line in text.lines() {
        if !line.contains("Playback") && !line.contains("Capture") {
            continue;
        }
        let Some(start) = line.find('[') else {
            continue;
        };
        let rest = &line[start + 1..];
        let Some(end) = rest.find('%') else {
            continue;
        };
        return rest[..end].parse().ok();
    }
    None
}

fn parse_amixer_muted(text: &str) -> bool {
    text.lines()
        .filter(|line| line.contains("Playback") || line.contains("Capture"))
        .any(|line| line.contains("[off]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WPCTL_STATUS: &str = "\
PipeWire 'pipewire-0' [1.6.2, user, cookie:2242213350]
 └─ Clients:
        33. pipewire                            [1.6.2, user, pid:929168]

Audio
 ├─ Devices:
 │      52. Built-in Audio Analog Stereo [alsa]
 │
 ├─ Sinks:
 │      66. Speaker One [vol: 1.00]
 │  *   69. Built-in Audio Speaker [vol: 0.76]
 │
 ├─ Sources:
 │      70. Built-in Audio Mic [vol: 1.00]
 │  *   71. Built-in Audio Digital Mic [vol: 0.50]
 │
 └─ Streams:

Video
 ├─ Devices:
 │      62. HP HD Camera [v4l2]
 │
 ├─ Sinks:
 │      87. HP HD Camera (V4L2)
 │
 └─ Sources:
";

    #[test]
    fn the_audio_section_is_found_and_the_video_section_is_not() {
        let section = section_of(WPCTL_STATUS, "Audio").expect("audio section");
        assert!(section.contains("Built-in Audio Speaker"));
        assert!(!section.contains("HP HD Camera"));
    }

    #[test]
    fn sinks_are_parsed_with_the_default_marked() {
        let section = section_of(WPCTL_STATUS, "Audio").expect("audio section");
        let (devices, default) = parse_wpctl_section(&section, "Sinks");

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "66");
        assert_eq!(devices[0].name, "Speaker One");
        assert!(!devices[0].default);
        assert_eq!(devices[1].id, "69");
        assert_eq!(devices[1].name, "Built-in Audio Speaker");
        assert!(devices[1].default);
        assert_eq!(default.as_deref(), Some("69"));
    }

    #[test]
    fn sources_are_parsed_separately_from_sinks() {
        let section = section_of(WPCTL_STATUS, "Audio").expect("audio section");
        let (devices, default) = parse_wpctl_section(&section, "Sources");

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "70");
        assert_eq!(devices[1].id, "71");
        assert!(devices[1].default);
        assert_eq!(default.as_deref(), Some("71"));
    }

    #[test]
    fn the_pipewire_volume_is_a_percentage() {
        assert_eq!(parse_wpctl_volume("Volume: 0.76"), Some(76));
        assert_eq!(parse_wpctl_volume("Volume: 1.00"), Some(100));
        assert_eq!(parse_wpctl_volume("Volume: 0"), Some(0));
        assert_eq!(parse_wpctl_volume("Volume: 0.756"), Some(76));
    }

    #[test]
    fn the_pipewire_mute_is_read_from_the_bracket() {
        assert!(parse_wpctl_mute("Volume: 0.76 [MUTED]"));
        assert!(!parse_wpctl_mute("Volume: 0.76"));
    }

    #[test]
    fn the_pulse_volume_is_read_from_the_percentage() {
        assert_eq!(
            parse_pactl_volume("Volume: front-left: 52429 /  80% / -3.00 dB"),
            Some(80)
        );
        assert_eq!(
            parse_pactl_volume("Volume: front-left: 65536 / 100%"),
            Some(100)
        );
    }

    #[test]
    fn the_alsa_volume_is_read_from_the_bracket() {
        let text = "Simple mixer control 'Master',0\n  Front Left: Playback 49808 [76%] [on]";
        assert_eq!(parse_amixer_volume(text), Some(76));
    }

    #[test]
    fn the_alsa_mute_is_read_from_the_off_marker() {
        let on = "  Front Left: Playback 49808 [76%] [on]";
        let off = "  Front Left: Playback 0 [0%] [off]";
        assert!(!parse_amixer_muted(on));
        assert!(parse_amixer_muted(off));
    }

    #[test]
    fn the_pulse_default_is_found_in_the_listing() {
        let text =
            "Sink #0\n\tName: alsa_output.pci-0000_00_1f.3.analog-stereo\n\tDescription: x\n";
        assert_eq!(
            pulse_default(text, "Sink").as_deref(),
            Some("alsa_output.pci-0000_00_1f.3.analog-stereo")
        );
    }

    #[test]
    fn a_missing_section_is_not_an_error() {
        assert!(section_of("Audio\n ├─ Sinks:\n", "Nope").is_none());
    }
}
