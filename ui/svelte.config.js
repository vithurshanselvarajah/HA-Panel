import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // The kiosk runs on a WebKitGTK that may not have the newest syntax
    // support, and the app is a one-shot build rather than something that
    // benefits from dev-time checks in production.
    runes: true,
  },
};
