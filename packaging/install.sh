#!/bin/sh

set -eu

PREFIX="${PREFIX:-/usr/bin}"
CONFIG_DIR="${CONFIG_DIR:-/etc/ha-panel}"
SKIP_SETUP="${SKIP_SETUP:-0}"
SKIP_SERVICE="${SKIP_SERVICE:-0}"
INSTALL_COMPOSITOR="${INSTALL_COMPOSITOR:-auto}"
HA_URL="${HA_URL:-}"
DEVICE_NAME="${DEVICE_NAME:-}"

log() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need_root() {
	if [ "$(id -u)" -ne 0 ]; then
		die "this script has to run as root"
	fi
}

detect_os() {
	if [ -r /etc/os-release ]; then
		. /etc/os-release
		OS_ID="${ID:-unknown}"
	else
		OS_ID="unknown"
	fi
}

setup_questions() {
	if [ "$SKIP_SETUP" = "1" ]; then
		log "setup is skipped; edit $CONFIG_DIR/config.json by hand"
		return
	fi

	if [ -z "$HA_URL" ]; then
		printf 'Home Assistant address [http://homeassistant.local:8123]: '
		read -r answer
		HA_URL="${answer:-http://homeassistant.local:8123}"
	fi

	if [ -z "$DEVICE_NAME" ]; then
		printf 'Name for this panel [ha-panel]: '
		read -r answer
		DEVICE_NAME="${answer:-ha-panel}"
	fi

	mkdir -p "$CONFIG_DIR"
	chown root:ha-panel "$CONFIG_DIR" 2>/dev/null || true
	chmod 0770 "$CONFIG_DIR" 2>/dev/null || true

	if [ ! -f "$CONFIG_DIR/config.json" ]; then
		cat >"$CONFIG_DIR/config.json" <<EOF
{
  "ha": {
    "url": "$HA_URL",
    "dashboard_path": "/lovelace",
    "fullscreen": true,
    "reload_on_reconnect": true
  },
  "kiosk": {
    "handle_width": 12,
    "drawer_width": 380,
    "show_handle": true,
    "on_screen_keyboard": true
  },
  "audio": {
    "backend": "auto",
    "volume": 50,
    "muted": false
  },
  "display": {
    "screen_power": "auto"
  },
  "network": {
    "wifi_enabled": true
  },
  "echomuse": {
    "enabled": false,
    "board": "$DEVICE_NAME"
  }
}
EOF
		chown root:ha-panel "$CONFIG_DIR/config.json" 2>/dev/null || true
		chmod 0660 "$CONFIG_DIR/config.json" 2>/dev/null || true
		log "wrote $CONFIG_DIR/config.json"
	else
		log "$CONFIG_DIR/config.json already exists; leaving it alone"
	fi
}

setup_service() {
	if [ "$SKIP_SERVICE" = "1" ]; then
		log "the service was not started"
		return
	fi

	case "$OS_ID" in
		debian | ubuntu | raspbian)
			if ! command -v systemctl >/dev/null 2>&1; then
				warn "systemd is not available; the service was not started"
				return
			fi

			systemctl daemon-reload

			case "$INSTALL_COMPOSITOR" in
			yes)
				systemctl enable ha-panel.target
				systemctl restart ha-panel.target
				;;
			no)
				systemctl enable ha-panel.service
				systemctl restart ha-panel.service
				;;
			auto)

				if [ -n "${XDG_CURRENT_DESKTOP:-}" ] || [ -n "${XDG_SESSION_DESKTOP:-}" ]; then
					systemctl enable ha-panel.service
					systemctl restart ha-panel.service
				else
					systemctl enable ha-panel.target
					systemctl restart ha-panel.target
				fi
				;;
			esac

			log "the service is running"
			;;
		alpine)
			if ! command -v rc-update >/dev/null 2>&1; then
				warn "OpenRC is not available; the service was not started"
				return
			fi
			rc-update add ha-panel default
			rc-service ha-panel restart
			log "the service is running"
			;;
		*)
			warn "unknown system; the service was not started"
			;;
	esac
}

main() {
	need_root
	detect_os

	log "installing HA-Panel"

	if [ ! -x "$PREFIX/ha-panel" ]; then
		die "$PREFIX/ha-panel is missing; install the package first"
	fi

	if ! getent group ha-panel >/dev/null 2>&1; then
		addgroup --system ha-panel 2>/dev/null || addgroup -S ha-panel
	fi
	if ! getent passwd ha-panel >/dev/null 2>&1; then
		adduser --system --ingroup ha-panel --home /var/lib/ha-panel \
			--no-create-home --disabled-password \
			--shell /usr/sbin/nologin ha-panel 2>/dev/null ||
			adduser -S -D -G ha-panel -H -h /var/lib/ha-panel -s /sbin/nologin ha-panel
	fi

	mkdir -p /var/lib/ha-panel /var/cache/ha-panel
	chown -R ha-panel:ha-panel /var/lib/ha-panel /var/cache/ha-panel

	setup_questions
	setup_service

	log "done"
}

main "$@"
