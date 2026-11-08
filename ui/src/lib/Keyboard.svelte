
<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "./api";

  const rows = [
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "-"],
  ];

  let shift = $state(false);
  let caps = $state(false);

  function label(key: string): string {
    if (shift || caps) {
      return key.toUpperCase();
    }
    return key;
  }

  async function press(key: string) {
    await api.sendKey(label(key), false).catch(() => {

    });

    if (shift) {
      shift = false;
    }
  }

  async function backspace() {
    await api.sendKey("", true).catch(() => {});
  }

  async function space() {
    await api.sendKey(" ", false).catch(() => {});
  }

  onMount(() => {

    const stop = (event: Event) => event.preventDefault();
    window.addEventListener("contextmenu", stop);
    return () => window.removeEventListener("contextmenu", stop);
  });
</script>

<div class="keyboard">
  <div class="row">
    <button class="key wide" class:on={shift} onclick={() => (shift = !shift)}>
      Shift
    </button>
    <button class="key wide" class:on={caps} onclick={() => (caps = !caps)}>
      Caps
    </button>
  </div>

  {#each rows as row, index (index)}
    <div class="row" class:offset={index === 2}>
      {#each row as key (key)}
        <button class="key" onclick={() => press(key)}>{label(key)}</button>
      {/each}
    </div>
  {/each}

  <div class="row">
    <button class="key wide" onclick={backspace}>⌫</button>
    <button class="key space" onclick={space}>space</button>
    <button class="key wide" onclick={() => (shift = false, (caps = false))}>
      Clear
    </button>
  </div>
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: #0e1118;
    overflow: hidden;
  }

  .keyboard {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.5rem;
    height: 100vh;
    box-sizing: border-box;
    justify-content: center;
    user-select: none;
  }

  .row {
    display: flex;
    gap: 0.3rem;
    justify-content: center;
  }

  .row.offset {
    padding: 0 1.6rem;
  }

  .key {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0.5rem 0;
    background: #1c2231;
    border: 1px solid #2c3446;
    border-radius: 6px;
    color: #e8ecf3;
    font-size: 1rem;
    cursor: pointer;
  }

  .key:active {
    background: #1565c0;
  }

  .key.on {
    background: #1565c0;
    border-color: #1976d2;
  }

  .key.wide {
    flex: 1.6;
    font-size: 0.8rem;
  }

  .key.space {
    flex: 4;
    font-size: 0.8rem;
  }
</style>
