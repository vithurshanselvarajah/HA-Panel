package rpc

import (
	"context"
	"encoding/json"
	"net"
	"path/filepath"
	"testing"
	"time"
)

func TestTheSocketServesMoreThanOneConnection(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sidecar.sock")

	server := NewServer()
	server.Register("ping", func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	served := make(chan error, 1)
	go func() {
		served <- server.ServeSocket(ctx, path)
	}()

	// Two connections, one after the other. A tool that connects once
	// and closes must not take the socket with it: the next tool has to
	// find it there.
	for attempt := 0; attempt < 2; attempt++ {
		conn := dialWithRetry(t, path)
		if conn == nil {
			t.Fatalf("the socket never appeared (attempt %d)", attempt)
		}

		if _, err := conn.Write([]byte("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n")); err != nil {
			t.Fatalf("cannot write: %v", err)
		}

		buffer := make([]byte, 256)
		n, err := conn.Read(buffer)
		if err != nil {
			t.Fatalf("cannot read: %v", err)
		}

		var response Response
		if err := json.Unmarshal(buffer[:n], &response); err != nil {
			t.Fatalf("the response is not JSON: %q", buffer[:n])
		}
		if response.Error != nil {
			t.Fatalf("the call failed: %v", response.Error)
		}

		conn.Close()
	}

	cancel()
	select {
	case <-served:
	case <-time.After(2 * time.Second):
		t.Error("the server did not stop")
	}
}

func dialWithRetry(t *testing.T, path string) net.Conn {
	t.Helper()

	for i := 0; i < 100; i++ {
		conn, err := net.Dial("unix", path)
		if err == nil {
			return conn
		}
		time.Sleep(20 * time.Millisecond)
	}
	return nil
}
