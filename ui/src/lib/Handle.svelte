
<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "./api";

  onMount(async () => {
    try {
      await api.openDrawer();
    } catch {

    }
  });
</script>

<button class="handle" aria-label="Open the controls"></button>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }

  .handle {
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    cursor: pointer;
  }

  .handle::after {
    content: "";
    display: block;
    width: 3px;
    height: 40%;
    margin: 30% auto;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.18);
    transition: background 120ms ease;
  }

  .handle:hover::after,
  .handle:active::after {
    background: rgba(255, 255, 255, 0.55);
  }
</style>
