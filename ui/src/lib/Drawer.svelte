
<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "./api";
  import type {
    AudioState,
    Config,
    DisplayState,
    DrawerState,
    WifiNetwork,
    WifiState,
  } from "./api";

  let panel = $state<DrawerState | null>(null);
  let audio = $state<AudioState | null>(null);
  let display = $state<DisplayState | null>(null);
  let wifi = $state<WifiState | null>(null);

  let tab = $state<"controls" | "settings" | "network" | "logs">("controls");
  let busy = $state(false);
  let message = $state("");
  let messageIsError = $state(false);

  let wifiNetworks = $state<WifiNetwork[]>([]);
  let scanning = $state(false);
  let selectedNetwork = $state<WifiNetwork | null>(null);
  let wifiPassword = $state("");

  let logs = $state("");
  let draft = $state<Config | null>(null);

  function say(text: string, isError = false) {
    message = text;
    messageIsError = isError;
    setTimeout(() => {
      if (message === text) {
        message = "";
      }
    }, 4000);
  }

  async function act(action: () => Promise<void>, failure: string) {
    busy = true;
    try {
      await action();
      messageIsError = false;
    } catch (err) {
      say(`${failure}: ${String(err)}`, true);
    } finally {
      busy = false;
    }
  }

  async function refresh() {
    try {
      const [next, nextAudio, nextDisplay, nextWifi] = await Promise.all([
        api.getState(),
        api.getAudioState(),
        api.getDisplayState(),
        api.getWifiState(),
      ]);
      panel = next;
      audio = nextAudio;
      display = nextDisplay;
      wifi = nextWifi;
      wifiNetworks = nextWifi.networks;
      draft = structuredClone(next.config);
    } catch (err) {
      say(`cannot read the device state: ${String(err)}`, true);
    }
  }

  onMount(refresh);

  async function changeVolume(volume: number) {
    if (!panel) return;
    panel.config.audio.volume = volume;
    await act(() => api.setVolume(volume), "cannot set the volume");
    await refresh();
  }

  async function toggleMute() {
    if (!panel) return;
    const muted = !panel.config.audio.muted;
    panel.config.audio.muted = muted;
    await act(() => api.setMuted(muted), "cannot change the mute state");
    await refresh();
  }

  async function chooseOutput(device: string) {
    if (!panel) return;
    panel.config.audio.output_device = device;
    await act(() => api.setOutputDevice(device), "cannot change the output device");
    await refresh();
  }

  async function chooseInput(device: string) {
    if (!panel) return;
    panel.config.audio.input_device = device;
    await act(() => api.setInputDevice(device), "cannot change the input device");
    await refresh();
  }

  async function changeBrightness(brightness: number) {
    if (!panel) return;
    panel.config.display.brightness = brightness;
    await act(() => api.setBrightness(brightness), "cannot set the brightness");
    await refresh();
  }

  async function saveSettings() {
    if (!draft) return;
    await act(async () => {
      await api.saveConfig(draft as Config);
      say("settings saved");
    }, "cannot save the settings");
    await refresh();
  }

  async function scan() {
    scanning = true;
    try {
      wifiNetworks = await api.scanWifi();
    } catch (err) {
      say(`cannot scan: ${String(err)}`, true);
    } finally {
      scanning = false;
    }
  }

  async function connect() {
    if (!selectedNetwork) return;
    await act(async () => {
      await api.joinWifi(
        selectedNetwork!.ssid,
        selectedNetwork!.secured ? wifiPassword : null,
      );
      say(`joined ${selectedNetwork!.ssid}`);
      selectedNetwork = null;
      wifiPassword = "";
    }, "cannot join the network");
    await refresh();
  }

  async function forget(network: WifiNetwork) {
    await act(async () => {
      await api.forgetWifi(network.ssid);
      say(`forgot ${network.ssid}`);
    }, "cannot forget the network");
    await refresh();
  }

  async function toggleRadio(on: boolean) {
    await act(() => api.setWifiRadio(on), "cannot change the radio");
    await refresh();
  }

  async function loadLogs() {
    try {
      logs = await api.getLogs("info");
    } catch (err) {
      say(`cannot read the log: ${String(err)}`, true);
    }
  }

  async function clearLogs() {
    await api.clearLogs();
    logs = "";
  }
</script>

<div class="drawer">
  <header>
    <h1>Home Panel</h1>
    {#if panel}
      <span class="host">{panel.status.hostname}</span>
    {/if}
  </header>

  <nav>
    <button class:active={tab === "controls"} onclick={() => (tab = "controls")}>
      Controls
    </button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>
      Settings
    </button>
    <button
      class:active={tab === "network"}
      onclick={() => {
        tab = "network";
        refresh();
      }}
    >
      Network
    </button>
    <button
      class:active={tab === "logs"}
      onclick={() => {
        tab = "logs";
        loadLogs();
      }}
    >
      Logs
    </button>
  </nav>

  {#if message}
    <p class="message" class:error={messageIsError}>{message}</p>
  {/if}

  <div class="body">
    {#if tab === "controls"}
      <section>
        <h2>Sound</h2>
        {#if audio && !audio.available}
          <p class="hint">No audio backend was found on this device.</p>
        {:else if audio}
          <label>
            <span>Volume</span>
            <input
              type="range"
              min="0"
              max="100"
              value={panel?.config.audio.volume ?? 50}
              disabled={busy}
              oninput={(e) => changeVolume(Number(e.currentTarget.value))}
            />
            <output>{panel?.config.audio.volume ?? 0}%</output>
          </label>

          <button class="wide" disabled={busy} onclick={toggleMute}>
            {panel?.config.audio.muted ? "Unmute" : "Mute"}
          </button>

          {#if audio.outputs.length > 1}
            <label>
              <span>Output</span>
              <select
                value={panel?.config.audio.output_device ?? audio.selected_output ?? ""}
                onchange={(e) => chooseOutput(e.currentTarget.value)}
              >
                {#each audio.outputs as device (device.id)}
                  <option value={device.id}>{device.name}</option>
                {/each}
              </select>
            </label>
          {/if}

          {#if audio.inputs.length > 1}
            <label>
              <span>Input</span>
              <select
                value={panel?.config.audio.input_device ?? audio.selected_input ?? ""}
                onchange={(e) => chooseInput(e.currentTarget.value)}
              >
                {#each audio.inputs as device (device.id)}
                  <option value={device.id}>{device.name}</option>
                {/each}
              </select>
            </label>
          {/if}

          <p class="hint">Backend: {audio.backend}</p>
        {/if}
      </section>

      <section>
        <h2>Screen</h2>
        <div class="row">
          <button
            disabled={busy || !display?.screen_power_supported}
            onclick={() => act(() => api.screenOff(), "cannot turn the screen off")}
          >
            Turn off
          </button>
          <button
            disabled={busy || !display?.screen_power_supported}
            onclick={() => act(() => api.screenOn(), "cannot turn the screen on")}
          >
            Turn on
          </button>
        </div>

        {#if display && !display.screen_power_supported}
          <p class="hint">
            Screen power is not supported here. Set a command in the settings if
            this device needs one.
          </p>
        {:else if display}
          <p class="hint">Mechanism: {display.screen_power_mechanism}</p>
        {/if}

        {#if display && display.brightness_supported}
          <label>
            <span>Brightness</span>
            <input
              type="range"
              min="0"
              max="100"
              value={panel?.config.display.brightness ?? display.brightness ?? 50}
              disabled={busy}
              oninput={(e) => changeBrightness(Number(e.currentTarget.value))}
            />
            <output>{panel?.config.display.brightness ?? display.brightness ?? 0}%</output>
          </label>
        {:else if display}
          <p class="hint">
            Brightness is not supported here. Set a command in the settings if
            this device needs one.
          </p>
        {/if}
      </section>
    {:else if tab === "settings" && draft}
      <section>
        <h2>Home Assistant</h2>
        <label>
          <span>Address</span>
          <input
            type="url"
            bind:value={draft.ha.url}
            placeholder="http://homeassistant.local:8123"
          />
        </label>
        <label>
          <span>Dashboard path</span>
          <input type="text" bind:value={draft.ha.dashboard_path} />
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={draft.ha.fullscreen} />
          <span>Fullscreen</span>
        </label>
      </section>

      <section>
        <h2>Display</h2>
        <label>
          <span>Screen power</span>
          <select bind:value={draft.display.screen_power}>
            <option value="auto">Automatic</option>
            <option value="wlopm">wlopm</option>
            <option value="xset">xset</option>
            <option value="disabled">Disabled</option>
          </select>
        </label>
        <label>
          <span>Brightness command</span>
          <input type="text" bind:value={draft.display.brightness_command} />
        </label>
        <label>
          <span>Screen off command</span>
          <input type="text" bind:value={draft.display.screen_off_command} />
        </label>
        <label>
          <span>Screen on command</span>
          <input type="text" bind:value={draft.display.screen_on_command} />
        </label>
      </section>

      <section>
        <h2>Voice</h2>
        <label class="check">
          <input type="checkbox" bind:checked={draft.echomuse.enabled} />
          <span>Enable the EchoMuse voice link</span>
        </label>
        <label>
          <span>Controller</span>
          <input
            type="text"
            bind:value={draft.echomuse.controller_url}
            placeholder="discovered automatically"
          />
        </label>
        <label>
          <span>Controller TLS port</span>
          <input type="number" bind:value={draft.echomuse.controller_tls_port} min="0" />
        </label>
        <label>
          <span>Device name</span>
          <input type="text" bind:value={draft.echomuse.board} />
        </label>
      </section>

      <section>
        <h2>Kiosk</h2>
        <label class="check">
          <input type="checkbox" bind:checked={draft.kiosk.show_handle} />
          <span>Show the edge handle</span>
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={draft.kiosk.on_screen_keyboard} />
          <span>On-screen keyboard</span>
        </label>
        <label>
          <span>Screen off after (seconds, 0 disables)</span>
          <input type="number" bind:value={draft.kiosk.screen_off_timeout_secs} min="0" />
        </label>
      </section>

      <button class="wide primary" disabled={busy} onclick={saveSettings}>
        Save
      </button>
    {:else if tab === "network"}
      <section>
        <h2>Wi-Fi</h2>
        {#if !wifi || !wifi.available}
          <p class="hint">NetworkManager was not found on this device.</p>
        {:else if !wifi.radio_present}
          <p class="hint">This device has no Wi-Fi radio.</p>
        {:else}
          <div class="row">
            <button
              class="wide"
              disabled={busy || scanning}
              onclick={() => toggleRadio(!(wifi?.radio_on ?? false))}
            >
              {wifi?.radio_on ? "Turn the radio off" : "Turn the radio on"}
            </button>
            <button class="wide" disabled={busy || scanning} onclick={scan}>
              {scanning ? "Scanning…" : "Scan"}
            </button>
          </div>

          {#if wifi.connected_ssid}
            <p class="hint">Connected to {wifi.connected_ssid}</p>
          {/if}

          <ul class="networks">
            {#each wifiNetworks as network (network.ssid)}
              <li>
                <button class="network" onclick={() => (selectedNetwork = network)}>
                  <span class="ssid">{network.ssid}</span>
                  <span class="meta">
                    {network.signal}%
                    {#if network.connected}· connected{/if}
                    {#if network.secured}· secured{/if}
                  </span>
                </button>
                {#if network.connected}
                  <button class="forget" onclick={() => forget(network)}>Forget</button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else if tab === "logs"}
      <section>
        <h2>Log</h2>
        <div class="row">
          <button class="wide" onclick={loadLogs}>Refresh</button>
          <button class="wide" onclick={clearLogs}>Clear</button>
        </div>
        <pre>{logs || "The log is empty."}</pre>
        {#if panel}
          <p class="hint">
            {panel.status.log_records} records held in memory. Nothing is written to the
            disk unless something fails.
          </p>
        {/if}
      </section>
    {/if}
  </div>

  {#if selectedNetwork}
    <div class="overlay">
      <div class="sheet">
        <h2>{selectedNetwork.ssid}</h2>
        {#if selectedNetwork.secured}
          <input
            type="password"
            placeholder="Password"
            bind:value={wifiPassword}
            onkeydown={(e) => e.key === "Enter" && connect()}
          />
        {:else}
          <p class="hint">This network is open.</p>
        {/if}
        <div class="row">
          <button onclick={() => (selectedNetwork = null)}>Cancel</button>
          <button class="primary" onclick={connect}>Connect</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: #10131a;
    color: #e8ecf3;
    font-family:
      system-ui,
      -apple-system,
      "Segoe UI",
      sans-serif;
    overscroll-behavior: none;
  }

  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #10131a;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 0.75rem 1rem 0.5rem;
    border-bottom: 1px solid #232838;
  }

  h1 {
    font-size: 1.05rem;
    margin: 0;
    font-weight: 600;
  }

  .host {
    font-size: 0.8rem;
    color: #8a93a8;
  }

  nav {
    display: flex;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #232838;
  }

  nav button {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    color: #8a93a8;
    padding: 0.45rem 0.25rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  nav button.active {
    background: #1c2231;
    color: #e8ecf3;
    border-color: #2c3446;
  }

  .message {
    margin: 0;
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
    background: #16301f;
    color: #9be5b5;
  }

  .message.error {
    background: #3a1a1a;
    color: #f0a3a3;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 1rem 2rem;
  }

  section {
    margin-bottom: 1.25rem;
  }

  h2 {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #6f7891;
    margin: 1rem 0 0.5rem;
  }

  label {
    display: grid;
    grid-template-columns: 8.5rem 1fr;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
    font-size: 0.85rem;
  }

  label > span {
    color: #a6aec2;
  }

  label.check {
    grid-template-columns: 1.2rem 1fr;
  }

  input[type="text"],
  input[type="url"],
  input[type="password"],
  input[type="number"],
  select {
    width: 100%;
    padding: 0.45rem 0.5rem;
    background: #1a2030;
    border: 1px solid #2c3446;
    border-radius: 6px;
    color: #e8ecf3;
    font-size: 0.85rem;
    box-sizing: border-box;
  }

  input[type="range"] {
    width: 100%;
    accent-color: #2196f3;
  }

  output {
    font-size: 0.8rem;
    color: #8a93a8;
    min-width: 2.5rem;
    text-align: right;
  }

  button {
    background: #1c2231;
    border: 1px solid #2c3446;
    color: #e8ecf3;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  button.primary {
    background: #1565c0;
    border-color: #1976d2;
  }

  button.wide {
    width: 100%;
    margin-bottom: 0.5rem;
  }

  .row {
    display: flex;
    gap: 0.5rem;
  }

  .row button {
    flex: 1;
  }

  .hint {
    font-size: 0.8rem;
    color: #8a93a8;
    margin: 0.4rem 0;
  }

  .networks {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
  }

  .networks li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
  }

  .network {
    flex: 1;
    display: flex;
    justify-content: space-between;
    text-align: left;
    gap: 0.5rem;
  }

  .meta {
    color: #8a93a8;
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .forget {
    font-size: 0.78rem;
    padding: 0.4rem 0.6rem;
  }

  pre {
    background: #0b0e14;
    border: 1px solid #232838;
    border-radius: 6px;
    padding: 0.6rem;
    font-size: 0.72rem;
    line-height: 1.4;
    max-height: 60vh;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(6, 8, 12, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
  }

  .sheet {
    background: #161b27;
    border: 1px solid #2c3446;
    border-radius: 10px;
    padding: 1rem;
    width: 100%;
  }

  .sheet h2 {
    margin-top: 0;
  }
</style>
