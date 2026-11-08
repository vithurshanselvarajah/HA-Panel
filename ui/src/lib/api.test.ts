import { describe, expect, it } from "vitest";

interface HaConfig {
  url: string;
  dashboard_path: string;
  fullscreen: boolean;
  reload_on_reconnect: boolean;
}

interface Config {
  ha: HaConfig;
  kiosk: { handle_width: number; drawer_width: number };
  audio: { backend: string; volume: number; muted: boolean };
  echomuse: { enabled: boolean; controller_url: string | null };
}

describe("the configuration shape", () => {
  it("uses the snake_case names the Rust side serialises", () => {

    const config: Config = {
      ha: {
        url: "http://ha.local:8123",
        dashboard_path: "/lovelace",
        fullscreen: true,
        reload_on_reconnect: true,
      },
      kiosk: { handle_width: 12, drawer_width: 380 },
      audio: { backend: "auto", volume: 50, muted: false },
      echomuse: { enabled: false, controller_url: null },
    };

    expect(Object.keys(config.ha)).toContain("dashboard_path");
    expect(Object.keys(config.kiosk)).toContain("handle_width");
    expect(Object.keys(config.audio)).toContain("backend");
    expect(Object.keys(config.echomuse)).toContain("controller_url");
  });

  it("carries a null for an unset device rather than an empty string", () => {

    const config: Config = {
      ha: {
        url: "http://ha.local:8123",
        dashboard_path: "/lovelace",
        fullscreen: true,
        reload_on_reconnect: true,
      },
      kiosk: { handle_width: 12, drawer_width: 380 },
      audio: { backend: "auto", volume: 50, muted: false },
      echomuse: { enabled: false, controller_url: null },
    };

    expect(config.echomuse.controller_url).toBeNull();
  });
});
