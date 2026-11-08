
import { mount } from "svelte";
import Drawer from "./lib/Drawer.svelte";
import Handle from "./lib/Handle.svelte";

const label = window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label ?? "drawer";

const target = document.getElementById("app");
if (!target) {
  throw new Error("the app element is missing");
}

if (label === "handle") {
  mount(Handle, { target });
} else {
  mount(Drawer, { target });
}
