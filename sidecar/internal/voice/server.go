package voice

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"os"
	"sync"
	"time"

	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse"
	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse/display"
	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse/protocol"
	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse/state"

	"github.com/vithurshanselvarajah/ha-panel/sidecar/internal/audio"
	"github.com/vithurshanselvarajah/ha-panel/sidecar/internal/rpc"
)

const (
	MethodPing = "ping"

	MethodState = "state"

	MethodCapabilities = "capabilities"

	MethodReconnect = "reconnect"

	MethodStartListening = "startListening"

	MethodStopListening = "stopListening"

	MethodSetVolume = "setVolume"

	MethodSetMuted = "setMuted"

	MethodRing = "ring"

	MethodStateChanged = "ha-panel://stateChanged"
)

type Settings struct {
	DeviceID string `json:"device_id"`

	Board string `json:"board"`

	Version string `json:"version"`

	ControllerURL string `json:"controller_url"`

	ControllerTLSPort int `json:"controller_tls_port"`

	CredentialsDir string `json:"credentials_dir"`

	InputDevice string `json:"input_device"`

	OutputDevice string `json:"output_device"`

	SocketPath string `json:"socket_path"`
}

type VoiceState struct {
	Connected bool `json:"connected"`

	Approved bool `json:"approved"`

	Phase string `json:"phase"`

	PhaseReason string `json:"phase_reason"`

	Listening bool `json:"listening"`

	Speaking bool `json:"speaking"`

	Thinking bool `json:"thinking"`

	Muted bool `json:"muted"`

	Volume int `json:"volume"`

	Capabilities []string `json:"capabilities"`

	LastError string `json:"last_error,omitempty"`
}

type Server struct {
	settings Settings
	logger   *slog.Logger

	mu sync.RWMutex

	device *echomuse.Device

	snapshot state.Snapshot

	ring *display.Tracker

	backend *audio.Backend

	capture io.ReadCloser

	captureCancel context.CancelFunc

	playback io.WriteCloser

	volume int

	muted bool

	notify func(VoiceState)

	rpc *rpc.Server
}

func NewServer(settings Settings) (*Server, error) {
	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{

		Level: slog.LevelInfo,
	}))

	server := &Server{
		settings: settings,
		logger:   logger,
		ring:     display.NewTracker(),
		volume:   50,
		rpc:      rpc.NewServer(),
	}

	server.registerMethods()

	return server, nil
}

func (s *Server) Notify(fn func(VoiceState)) {
	s.mu.Lock()
	s.notify = fn
	s.mu.Unlock()
}

func (s *Server) registerMethods() {
	s.rpc.Register(MethodPing, func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})

	s.rpc.Register(MethodState, func(ctx context.Context, params json.RawMessage) (any, error) {
		return s.VoiceState(), nil
	})

	s.rpc.Register(MethodCapabilities, func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"capabilities": s.Capabilities()}, nil
	})

	s.rpc.Register(MethodReconnect, func(ctx context.Context, params json.RawMessage) (any, error) {
		s.logger.Info("reconnecting to the controller")

		if device := s.deviceInstance(); device != nil {
			device.Stop()
		}
		return map[string]any{"reconnecting": true}, nil
	})

	s.rpc.Register(MethodStartListening, func(ctx context.Context, params json.RawMessage) (any, error) {
		if err := s.StartCapture(ctx); err != nil {
			return nil, err
		}
		return map[string]any{"listening": true}, nil
	})

	s.rpc.Register(MethodStopListening, func(ctx context.Context, params json.RawMessage) (any, error) {
		s.StopCapture()
		return map[string]any{"listening": false}, nil
	})

	s.rpc.Register(MethodSetVolume, func(ctx context.Context, params json.RawMessage) (any, error) {
		var request struct {
			Level int `json:"level"`
		}
		if err := decode(params, &request); err != nil {
			return nil, err
		}
		s.SetVolume(request.Level)
		return map[string]any{"volume": request.Level}, nil
	})

	s.rpc.Register(MethodSetMuted, func(ctx context.Context, params json.RawMessage) (any, error) {
		var request struct {
			Muted bool `json:"muted"`
		}
		if err := decode(params, &request); err != nil {
			return nil, err
		}
		s.SetMuted(request.Muted)
		return map[string]any{"muted": request.Muted}, nil
	})

	s.rpc.Register(MethodRing, func(ctx context.Context, params json.RawMessage) (any, error) {
		return s.ring.View(), nil
	})
}

func decode(params json.RawMessage, target any) error {
	if len(params) == 0 {
		return rpc.ErrorInvalidParams
	}
	if err := json.Unmarshal(params, target); err != nil {
		return rpc.ErrorInvalidParams
	}
	return nil
}

func (s *Server) ServeStdio(ctx context.Context) error {
	return s.rpc.ServeStdio(ctx)
}

func (s *Server) ServeSocket(ctx context.Context, path string) error {
	return s.rpc.ServeSocket(ctx, path)
}

func (s *Server) Run(ctx context.Context) error {
	backend, err := audio.Open(s.settings.InputDevice, s.settings.OutputDevice)
	if err != nil {
		return fmt.Errorf("cannot open the audio backend: %w", err)
	}
	s.mu.Lock()
	s.backend = backend
	s.mu.Unlock()
	s.logger.Info("audio backend ready", "backend", backend.Describe())

	options := echomuse.Options{
		DeviceID:          s.settings.DeviceID,
		Version:           s.settings.Version,
		Board:             s.settings.Board,
		BaseOS:            protocol.BaseOSEmOS,
		Capabilities:      capabilities(),
		ControllerURL:     s.settings.ControllerURL,
		ControllerTLSPort: s.settings.ControllerTLSPort,
		CredentialsDir:    s.settings.CredentialsDir,
		DiscoverTimeout:   10 * time.Second,
		ReconnectDelay:    2 * time.Second,
		ReconnectMaxDelay: 60 * time.Second,
		Logger:            s.logger,
	}

	device, err := echomuse.New(options)
	if err != nil {
		return fmt.Errorf("cannot create the device: %w", err)
	}

	s.mu.Lock()
	s.device = device
	s.mu.Unlock()

	s.registerCallbacks(device)

	device.State().OnChange(func(snapshot state.Snapshot) {
		s.mu.Lock()
		s.snapshot = snapshot
		notify := s.notify
		s.mu.Unlock()

		if notify != nil {
			notify(s.VoiceState())
		}
	})

	if !device.Paired() {
		device.StartPairing()
	}

	err = device.Run(ctx)
	if err != nil && ctx.Err() == nil {
		return err
	}
	return nil
}

func (s *Server) registerCallbacks(device *echomuse.Device) {

	device.OnLEDs(s.ring.OnLEDs)
	device.OnLEDAnim(s.ring.OnLEDAnim)

	device.OnMicStart(func(lockMic bool) {
		s.logger.Info("the controller opened the microphone", "locked", lockMic)
		if err := s.StartCapture(context.Background()); err != nil {
			s.logger.Error("cannot open the microphone", "error", err)
		}
	})

	device.OnMicStop(func() {
		s.logger.Info("the controller closed the microphone")
		s.StopCapture()
	})

	device.OnSpeaker(func(pcm []byte) {
		s.play(pcm)
	})

	device.OnSpeakerEOS(func() {
		s.drainPlayback()
	})

	device.OnMusic(func(pcm []byte) {
		s.play(pcm)
	})

	device.OnMusicEOS(func() {
		s.drainPlayback()
	})

	device.OnVolumeSet(func(level int) {
		s.logger.Info("the controller set the volume", "level", level)
		s.SetVolume(level)
	})

	device.OnDuck(func(on bool) {
		s.logger.Info("the controller asked to duck", "ducked", on)
	})

	device.OnCue(func(cue string) {
		s.logger.Info("the controller sent a cue", "cue", cue)
	})
}

func (s *Server) StartCapture(parent context.Context) error {
	s.mu.Lock()
	if s.capture != nil {
		s.mu.Unlock()
		return nil
	}
	backend := s.backend
	device := s.device
	s.mu.Unlock()

	if backend == nil {
		return rpc.ErrorInvalidState
	}
	if device == nil {
		return rpc.ErrorInvalidState
	}

	reader, err := backend.Capture(parent, audio.CaptureRate)
	if err != nil {
		return fmt.Errorf("cannot open the microphone: %w", err)
	}

	ctx, cancel := context.WithCancel(parent)

	s.mu.Lock()
	s.capture = reader
	s.captureCancel = cancel
	s.mu.Unlock()

	go s.pumpCapture(ctx, device, reader)

	return nil
}

func (s *Server) StopCapture() {
	s.mu.Lock()
	reader := s.capture
	cancel := s.captureCancel
	s.capture = nil
	s.captureCancel = nil
	s.mu.Unlock()

	if cancel != nil {
		cancel()
	}
	if reader != nil {
		_ = reader.Close()
	}
}

func (s *Server) pumpCapture(ctx context.Context, device *echomuse.Device, reader io.ReadCloser) {

	buffer := make([]byte, audio.CaptureRate/10*2)

	for {
		n, err := reader.Read(buffer)
		if n > 0 {
			s.mu.RLock()
			muted := s.muted
			s.mu.RUnlock()

			if !muted {
				if sendErr := device.SendMicAudio(buffer[:n]); sendErr != nil {
					s.logger.Debug("the audio did not reach the controller", "error", sendErr)
					return
				}
			}
		}
		if err != nil {
			if ctx.Err() == nil {
				s.logger.Warn("the microphone stopped", "error", err)
			}
			return
		}
		if ctx.Err() != nil {
			return
		}
	}
}

func (s *Server) play(pcm []byte) {
	s.mu.Lock()
	if s.playback == nil {
		if s.backend == nil {
			s.mu.Unlock()
			return
		}
		writer, err := s.backend.Playback(context.Background(), audio.PlaybackRate)
		if err != nil {
			s.mu.Unlock()
			s.logger.Error("cannot open the speaker", "error", err)
			return
		}
		s.playback = writer
	}
	writer := s.playback
	s.mu.Unlock()

	if _, err := writer.Write(pcm); err != nil {
		s.logger.Debug("cannot play the audio", "error", err)
	}
}

func (s *Server) drainPlayback() {
	s.mu.Lock()
	writer := s.playback
	s.playback = nil
	s.mu.Unlock()

	if writer == nil {
		return
	}

	if drainer, ok := writer.(interface{ Drain() error }); ok {
		_ = drainer.Drain()
	}
	_ = writer.Close()
}

func (s *Server) SetVolume(level int) {
	if level < 0 {
		level = 0
	}
	if level > 100 {
		level = 100
	}

	s.mu.Lock()
	s.volume = level
	device := s.device
	s.mu.Unlock()

	if device != nil {
		_ = device.SendVolumeState(level)
	}
}

func (s *Server) SetMuted(muted bool) {
	s.mu.Lock()
	s.muted = muted
	device := s.device
	s.mu.Unlock()

	if device != nil {
		_ = device.SendMuteState(muted)
	}
}

func (s *Server) VoiceState() VoiceState {
	s.mu.RLock()
	defer s.mu.RUnlock()

	snapshot := s.snapshot
	caps := make([]string, 0, len(snapshot.Capabilities))
	for _, capability := range snapshot.Capabilities {
		caps = append(caps, string(capability))
	}

	return VoiceState{
		Connected:    snapshot.LinkUp,
		Approved:     snapshot.Approved,
		Phase:        string(snapshot.Phase),
		PhaseReason:  snapshot.PhaseReason,
		Listening:    snapshot.Listening,
		Speaking:     snapshot.Speaking,
		Thinking:     snapshot.Thinking,
		Muted:        s.muted,
		Volume:       s.volume,
		Capabilities: caps,
		LastError:    snapshot.LastError,
	}
}

func (s *Server) Capabilities() []string {
	s.mu.RLock()
	defer s.mu.RUnlock()

	caps := make([]string, 0, len(s.snapshot.Capabilities))
	for _, capability := range s.snapshot.Capabilities {
		caps = append(caps, string(capability))
	}
	return caps
}

func (s *Server) deviceInstance() *echomuse.Device {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.device
}

func capabilities() []protocol.Capability {
	return []protocol.Capability{
		protocol.CapMic,
		protocol.CapSpeaker,
		protocol.CapLEDs,
		protocol.CapLEDAnim,
		protocol.CapButtons,
		protocol.CapPairing,
		protocol.CapWakeCue,
		protocol.CapVolumeCue,
	}
}
