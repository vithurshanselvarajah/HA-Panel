package main

import (
	"context"
	"encoding/json"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"
)

// The sidecar has to keep serving its socket after its standard input
// closes. The app closes the standard input to ask the sidecar to stop,
// but a socket is a separate way in: a health check or a debugging shell
// may be using it, and the sidecar must not vanish from under them.
func TestTheSidecarKeepsServingItsSocketAfterStdinCloses(t *testing.T) {
	if testing.Short() {
		t.Skip("this builds and runs the sidecar")
	}

	dir := t.TempDir()
	socket := filepath.Join(dir, "sidecar.sock")

	binary := buildSidecar(t)

	command := exec.Command(binary, "--socket", socket)
	command.Stdin = nil
	command.Stdout = os.Stderr
	command.Stderr = os.Stderr

	if err := command.Start(); err != nil {
		t.Fatalf("cannot start the sidecar: %v", err)
	}
	defer func() {
		_ = command.Process.Kill()
		_ = command.Wait()
	}()

	// The socket has to appear.
	var conn net.Conn
	for i := 0; i < 150; i++ {
		dialed, err := net.Dial("unix", socket)
		if err == nil {
			conn = dialed
			break
		}
		time.Sleep(20 * time.Millisecond)
	}
	if conn == nil {
		t.Fatal("the socket never appeared")
	}
	conn.Close()

	// The standard input is already closed. The sidecar must still be
	// there, and the socket must still work.
	time.Sleep(500 * time.Millisecond)

	conn, err := net.Dial("unix", socket)
	if err != nil {
		t.Fatalf("the socket went away when the standard input closed: %v", err)
	}
	defer conn.Close()

	if _, err := conn.Write([]byte("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n")); err != nil {
		t.Fatalf("cannot write: %v", err)
	}

	buffer := make([]byte, 256)
	n, err := conn.Read(buffer)
	if err != nil {
		t.Fatalf("cannot read: %v", err)
	}

	var response struct {
		Result json.RawMessage `json:"result"`
		Error  json.RawMessage `json:"error"`
	}
	if err := json.Unmarshal(buffer[:n], &response); err != nil {
		t.Fatalf("the response is not JSON: %q", buffer[:n])
	}
	if len(response.Error) > 0 {
		t.Fatalf("the call failed: %s", response.Error)
	}
}

// buildSidecar builds the sidecar and returns the path to it.
func buildSidecar(t *testing.T) string {
	t.Helper()

	binary := filepath.Join(t.TempDir(), "ha-panel-sidecar")

	command := exec.Command("go", "build", "-o", binary, ".")
	command.Env = append(os.Environ(), "CGO_ENABLED=0")
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("cannot build the sidecar: %v\n%s", err, output)
	}

	return binary
}

var _ = context.Background
