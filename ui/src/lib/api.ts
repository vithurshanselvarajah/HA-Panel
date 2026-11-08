
import { invoke } from "@tauri-apps/api/core";

export type AudioBackend = "auto" | "pulse" | "pipewire" | "alsa";
export type ScreenPower = "auto" | "wlopm" | "xset" | "disabled";

export interface HaConfig {
  url: string;
  dashboard_path: string;
  fullscreen: boolean;
  reload_on_reconnect: boolean;
}

export interface KioskConfig {
  handle_width: number;
  drawer_width: number;
  show_handle: boolean;
  on_screen_keyboard: boolean;
  screen_off_timeout_secs: number;
  screen_on_timeout_secs: number;
}

export interface AudioConfig {
  backend: AudioBackend;
  volume: number;
  muted: boolean;
  output_device: string | null;
  input_device: string | null;
}

export interface DisplayConfig {
  screen_power: ScreenPower;
  brightness: number | null;
  brightness_command: string | null;
  screen_off_command: string | null;
  screen_on_command: string | null;
}

export interface NetworkConfig {
  wifi_enabled: boolean;
  preferred_connection: string | null;
}

export interface EchoMuseConfig {
  enabled: boolean;
  controller_url: string | null;
  controller_tls_port: number;
  device_id: string | null;
  board: string;
  version: string;
  credentials_dir: string;
  socket_path: string;
  input_device: string | null;
  output_device: string | null;
}

export interface Config {
  ha: HaConfig;
  kiosk: KioskConfig;
  audio: AudioConfig;
  display: DisplayConfig;
  network: NetworkConfig;
  echomuse: EchoMuseConfig;
}

export interface Status {
  version: string;
  hostname: string;
  display_server: string;
  has_desktop: boolean;
  sidecar_running: boolean;
  log_level: string;
  log_records: number;
}

export interface DrawerState {
  config: Config;
  status: Status;
  has_token: boolean;
}

export interface AudioDevice {
  id: string;
  name: string;
  default: boolean;
}

export interface AudioState {
  backend: string;
  available: boolean;
  volume: number;
  muted: boolean;
  outputs: AudioDevice[];
  inputs: AudioDevice[];
  selected_output: string | null;
  selected_input: string | null;
}

export interface Backlight {
  name: string;
  current: number;
  max: number;
}

export interface DisplayState {
  screen_power_supported: boolean;
  screen_power_mechanism: string;
  brightness_supported: boolean;
  backlights: Backlight[];
  brightness: number | null;
}

export interface WifiNetwork {
  ssid: string;
  signal: number;
  secured: boolean;
  connected: boolean;
}

export interface WifiState {
  available: boolean;
  radio_present: boolean;
  radio_on: boolean;
  interface: string | null;
  connected_ssid: string | null;
  networks: WifiNetwork[];
}

export function getState(): Promise<DrawerState> {
  return invoke("get_state");
}

export function saveConfig(config: Config): Promise<void> {
  return invoke("save_config", { config });
}

export function getAudioState(): Promise<AudioState> {
  return invoke("get_audio_state");
}

export function setVolume(volume: number): Promise<void> {
  return invoke("set_volume", { volume });
}

export function setMuted(muted: boolean): Promise<void> {
  return invoke("set_muted", { muted });
}

export function setOutputDevice(device: string): Promise<void> {
  return invoke("set_output_device", { device });
}

export function setInputDevice(device: string): Promise<void> {
  return invoke("set_input_device", { device });
}

export function getDisplayState(): Promise<DisplayState> {
  return invoke("get_display_state");
}

export function setBrightness(brightness: number): Promise<void> {
  return invoke("set_brightness", { brightness });
}

export function screenOff(): Promise<void> {
  return invoke("screen_off");
}

export function screenOn(): Promise<void> {
  return invoke("screen_on");
}

export function getWifiState(): Promise<WifiState> {
  return invoke("get_wifi_state");
}

export function scanWifi(): Promise<WifiNetwork[]> {
  return invoke("scan_wifi");
}

export function joinWifi(ssid: string, password: string | null): Promise<void> {
  return invoke("join_wifi", { ssid, password });
}

export function forgetWifi(ssid: string): Promise<void> {
  return invoke("forget_wifi", { ssid });
}

export function setWifiRadio(on: boolean): Promise<void> {
  return invoke("set_wifi_radio", { on });
}

export function openDrawer(): Promise<void> {
  return invoke("open_drawer");
}

export function closeDrawer(): Promise<void> {
  return invoke("close_drawer");
}

export function showKeyboard(): Promise<void> {
  return invoke("show_keyboard");
}

export function hideKeyboard(): Promise<void> {
  return invoke("hide_keyboard");
}

export function sendKey(text: string, backspace: boolean): Promise<void> {
  return invoke("send_key", { text, backspace });
}

export function getSidecarState(): Promise<unknown> {
  return invoke("get_sidecar_state");
}

export function startSidecar(): Promise<void> {
  return invoke("start_sidecar");
}

export function stopSidecar(): Promise<void> {
  return invoke("stop_sidecar");
}

export function getLogs(minLevel?: string): Promise<string> {
  return invoke("get_logs", { minLevel: minLevel ?? null });
}

export function clearLogs(): Promise<void> {
  return invoke("clear_logs");
}
