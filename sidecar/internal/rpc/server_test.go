package rpc

import (
	"context"
	"encoding/json"
	"strings"
	"testing"
)

func TestTheServerAnswersOnAStream(t *testing.T) {
	server := NewServer()
	server.Register("ping", func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})

	reader := strings.NewReader("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n")
	var out strings.Builder

	if err := server.serve(context.Background(), reader, &out); err != nil {
		t.Fatalf("the server failed: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(out.String()), "\n")
	if len(lines) != 1 {
		t.Fatalf("expected one response, got %d: %q", len(lines), out.String())
	}

	var response Response
	if err := json.Unmarshal([]byte(lines[0]), &response); err != nil {
		t.Fatalf("the response is not valid JSON: %v", err)
	}
	if response.Error != nil {
		t.Fatalf("the call failed: %v", response.Error)
	}
}

func TestTheServerAnswersSeveralRequests(t *testing.T) {
	server := NewServer()
	server.Register("ping", func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})

	reader := strings.NewReader(
		"{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n" +
			"{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":2}\n",
	)
	var out strings.Builder

	if err := server.serve(context.Background(), reader, &out); err != nil {
		t.Fatalf("the server failed: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(out.String()), "\n")
	if len(lines) != 2 {
		t.Fatalf("expected two responses, got %d: %q", len(lines), out.String())
	}
}

func TestALineThatIsNotJSONGetsAnError(t *testing.T) {
	server := NewServer()

	reader := strings.NewReader("this is not json\n")
	var out strings.Builder

	if err := server.serve(context.Background(), reader, &out); err != nil {
		t.Fatalf("the server failed: %v", err)
	}

	var response Response
	if err := json.Unmarshal([]byte(strings.TrimSpace(out.String())), &response); err != nil {
		t.Fatalf("the response is not valid JSON: %v", err)
	}
	if response.Error == nil {
		t.Fatal("a malformed line must produce an error")
	}
	if response.Error.Code != CodeInvalidParams {
		t.Errorf("the error has the wrong code: %d", response.Error.Code)
	}
}

func TestANotificationOnTheStreamGetsNoAnswer(t *testing.T) {
	server := NewServer()
	server.Register("ping", func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})

	reader := strings.NewReader("{\"jsonrpc\":\"2.0\",\"method\":\"ping\"}\n")
	var out strings.Builder

	if err := server.serve(context.Background(), reader, &out); err != nil {
		t.Fatalf("the server failed: %v", err)
	}

	if strings.TrimSpace(out.String()) != "" {
		t.Errorf("a notification must produce no answer: %q", out.String())
	}
}
