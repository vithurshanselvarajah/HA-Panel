
import { mount } from "svelte";
import Keyboard from "./lib/Keyboard.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("the app element is missing");
}

mount(Keyboard, { target });
