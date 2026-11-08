package rpc

import (
	"context"
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

func testServer() *Server {
	server := NewServer()
	server.Register("ping", func(ctx context.Context, params json.RawMessage) (any, error) {
		return map[string]any{"pong": true}, nil
	})
	server.Register("add", func(ctx context.Context, params json.RawMessage) (any, error) {
		var request struct {
			A int `json:"a"`
			B int `json:"b"`
		}
		if err := json.Unmarshal(params, &request); err != nil {
			return nil, ErrorInvalidParams
		}
		return request.A + request.B, nil
	})
	server.Register("boom", func(ctx context.Context, params json.RawMessage) (any, error) {
		return nil, errors.New("it broke")
	})
	return server
}

func TestPingIsAnswered(t *testing.T) {
	server := testServer()

	request := Request{JSONRPC: Version, Method: "ping", ID: json.RawMessage("1")}
	response := server.Handle(context.Background(), request)

	if response == nil {
		t.Fatal("a request must produce a response")
	}
	if response.Error != nil {
		t.Fatalf("the call failed: %v", response.Error)
	}
	if string(response.ID) != "1" {
		t.Errorf("the response has the wrong id: %s", response.ID)
	}

	var result map[string]any
	if err := json.Unmarshal(response.Result, &result); err != nil {
		t.Fatalf("the result is not valid JSON: %v", err)
	}
	if result["pong"] != true {
		t.Errorf("the result is wrong: %v", result)
	}
}

func TestAnUnknownMethodIsAnError(t *testing.T) {
	server := testServer()

	request := Request{JSONRPC: Version, Method: "nope", ID: json.RawMessage("1")}
	response := server.Handle(context.Background(), request)

	if response == nil {
		t.Fatal("a request must produce a response")
	}
	if response.Error == nil {
		t.Fatal("an unknown method must produce an error")
	}
	if response.Error.Code != CodeMethodNotFound {
		t.Errorf("the error has the wrong code: %d", response.Error.Code)
	}
}

func TestANotificationProducesNoResponse(t *testing.T) {
	server := testServer()

	request := Request{JSONRPC: Version, Method: "ping"}
	response := server.Handle(context.Background(), request)

	if response != nil {
		t.Errorf("a notification must produce no response: %+v", response)
	}
}

func TestAHandlerThatFailsBecomesAnError(t *testing.T) {
	server := testServer()

	request := Request{JSONRPC: Version, Method: "boom", ID: json.RawMessage("1")}
	response := server.Handle(context.Background(), request)

	if response == nil || response.Error == nil {
		t.Fatal("a failing handler must produce an error")
	}
	if response.Error.Code != CodeInternal {
		t.Errorf("the error has the wrong code: %d", response.Error.Code)
	}
	if !strings.Contains(response.Error.Message, "it broke") {
		t.Errorf("the error lost its message: %s", response.Error.Message)
	}
}

func TestParametersAreDecoded(t *testing.T) {
	server := testServer()

	request := Request{
		JSONRPC: Version,
		Method:  "add",
		Params:  json.RawMessage(`{"a": 2, "b": 3}`),
		ID:      json.RawMessage("1"),
	}
	response := server.Handle(context.Background(), request)

	if response.Error != nil {
		t.Fatalf("the call failed: %v", response.Error)
	}
	if string(response.Result) != "5" {
		t.Errorf("the result is wrong: %s", response.Result)
	}
}

func TestBadParametersAreAnInvalidParamsError(t *testing.T) {
	server := testServer()

	request := Request{
		JSONRPC: Version,
		Method:  "add",
		Params:  json.RawMessage(`not json`),
		ID:      json.RawMessage("1"),
	}
	response := server.Handle(context.Background(), request)

	if response == nil || response.Error == nil {
		t.Fatal("bad parameters must produce an error")
	}
	if response.Error.Code != CodeInvalidParams {
		t.Errorf("the error has the wrong code: %d", response.Error.Code)
	}
}
