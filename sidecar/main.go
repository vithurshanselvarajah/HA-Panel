package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/vithurshanselvarajah/ha-panel/sidecar/internal/voice"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "ha-panel-sidecar: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	var (
		socketPath  = flag.String("socket", "/run/ha-panel/sidecar.sock", "the Unix socket to serve JSON-RPC on")
		configPath  = flag.String("config", "", "a JSON file with the EchoMuse settings")
		configJSON  = flag.String("config-json", "", "the EchoMuse settings as JSON")
		deviceID    = flag.String("device-id", "", "the identity to present to the controller")
		board       = flag.String("board", "", "the hardware name the controller shows")
		version     = flag.String("version", "", "the firmware version the controller shows")
		controller  = flag.String("controller", "", "the controller's base URL; empty means discover it")
		tlsPort     = flag.Int("controller-tls-port", 0, "the controller's encrypted-WebSocket port")
		credDir     = flag.String("credentials", "", "where the link credentials live")
		inputDevice = flag.String("input", "", "the microphone to capture from")
		outputDev   = flag.String("output", "", "the speaker to play through")
	)
	flag.Parse()

	settings, err := loadSettings(*configPath, *configJSON, *deviceID, *board, *version, *controller, *tlsPort, *credDir, *inputDevice, *outputDev)
	if err != nil {
		return err
	}

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	server, err := voice.NewServer(settings)
	if err != nil {
		return err
	}

	if *socketPath != "" {
		go func() {
			if err := server.ServeSocket(ctx, *socketPath); err != nil {
				fmt.Fprintf(os.Stderr, "ha-panel-sidecar: the socket failed: %v\n", err)
			}
		}()
	}

	go func() {
		if err := server.ServeStdio(ctx); err != nil {
			fmt.Fprintf(os.Stderr, "ha-panel-sidecar: the standard stream failed: %v\n", err)
		}

		// The app closes the sidecar's standard input to ask it
		// to stop. The socket is a separate way in, though, so a
		// sidecar that is serving one keeps running: a health
		// check or a debugging shell may be using it.
		if *socketPath == "" {
			stop()
		}
	}()

	runErr := server.Run(ctx)

	time.Sleep(50 * time.Millisecond)

	return runErr
}

func loadSettings(
	configPath, configJSON, deviceID, board, version, controller string,
	tlsPort int, credDir, inputDevice, outputDevice string,
) (voice.Settings, error) {
	settings := voice.Settings{}

	if configPath != "" {
		bytes, err := os.ReadFile(configPath)
		if err != nil {
			return settings, fmt.Errorf("cannot read %s: %w", configPath, err)
		}
		if err := json.Unmarshal(bytes, &settings); err != nil {
			return settings, fmt.Errorf("cannot parse %s: %w", configPath, err)
		}
	}

	if configJSON != "" {
		if err := json.Unmarshal([]byte(configJSON), &settings); err != nil {
			return settings, fmt.Errorf("cannot parse the configuration: %w", err)
		}
	}

	if deviceID != "" {
		settings.DeviceID = deviceID
	}
	if board != "" {
		settings.Board = board
	}
	if version != "" {
		settings.Version = version
	}
	if controller != "" {
		settings.ControllerURL = controller
	}
	if tlsPort != 0 {
		settings.ControllerTLSPort = tlsPort
	}
	if credDir != "" {
		settings.CredentialsDir = credDir
	}
	if inputDevice != "" {
		settings.InputDevice = inputDevice
	}
	if outputDevice != "" {
		settings.OutputDevice = outputDevice
	}

	if settings.DeviceID == "" {
		settings.DeviceID = defaultDeviceID()
	}
	if settings.Board == "" {
		settings.Board = "ha-panel"
	}
	if settings.Version == "" {
		settings.Version = "0.1.0"
	}
	if settings.CredentialsDir == "" {
		settings.CredentialsDir = "/var/lib/ha-panel/echomuse"
	}

	return settings, nil
}

func defaultDeviceID() string {
	for _, path := range []string{"/etc/machine-id", "/var/lib/dbus/machine-id"} {
		if bytes, err := os.ReadFile(path); err == nil {
			id := trimSpace(string(bytes))
			if id != "" {
				return "ha-panel-" + id
			}
		}
	}

	hostname, err := os.Hostname()
	if err != nil || hostname == "" {
		return "ha-panel"
	}
	return "ha-panel-" + sanitize(hostname)
}

func trimSpace(s string) string {
	start := 0
	end := len(s)
	for start < end && isSpace(s[start]) {
		start++
	}
	for end > start && isSpace(s[end-1]) {
		end--
	}
	return s[start:end]
}

func isSpace(b byte) bool {
	return b == ' ' || b == '\t' || b == '\n' || b == '\r'
}

func sanitize(s string) string {
	out := make([]rune, 0, len(s))
	for _, r := range s {
		switch {
		case r >= 'a' && r <= 'z',
			r >= 'A' && r <= 'Z',
			r >= '0' && r <= '9',
			r == '-', r == '_':
			out = append(out, r)
		default:
			out = append(out, '-')
		}
	}
	return string(out)
}
