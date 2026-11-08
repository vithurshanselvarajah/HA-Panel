package audio

import (
	"context"
	"fmt"
	"io"

	sdkaudio "github.com/wilbowes/EchoMuse-SDK/clients/go/cmd/emulator/audio"
	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse/emulator"
	"github.com/wilbowes/EchoMuse-SDK/clients/go/echomuse/protocol"
)

var (
	CaptureRate = protocol.MicSampleRate

	PlaybackRate = protocol.SpeakerSampleRate
)

type Backend struct {
	inner emulator.AudioBackend
}

func Open(input, output string) (*Backend, error) {
	inner := sdkaudio.NewBackend(input, output)
	if inner == nil {
		return nil, fmt.Errorf("no audio backend is available")
	}
	return &Backend{inner: inner}, nil
}

func (b *Backend) Capture(ctx context.Context, rate int) (io.ReadCloser, error) {
	return b.inner.Capture(ctx, rate)
}

func (b *Backend) Playback(ctx context.Context, rate int) (io.WriteCloser, error) {
	return b.inner.Playback(ctx, rate)
}

func (b *Backend) Describe() string {
	return b.inner.Describe()
}

func (b *Backend) AudioDevices() ([]emulator.AudioDevice, []emulator.AudioDevice, error) {
	provider, ok := b.inner.(emulator.AudioDeviceProvider)
	if !ok {
		return nil, nil, fmt.Errorf("this backend cannot list its devices")
	}
	return provider.AudioDevices()
}

func (b *Backend) SelectedAudioDevices() (string, string) {
	provider, ok := b.inner.(emulator.AudioDeviceProvider)
	if !ok {
		return "", ""
	}
	return provider.SelectedAudioDevices()
}

func (b *Backend) SelectAudioDevices(input, output string) error {
	provider, ok := b.inner.(emulator.AudioDeviceProvider)
	if !ok {
		return fmt.Errorf("this backend cannot change its devices")
	}
	return provider.SelectAudioDevices(input, output)
}

func Devices() (inputs, outputs []emulator.AudioDevice, err error) {
	return sdkaudio.ListDevices()
}
