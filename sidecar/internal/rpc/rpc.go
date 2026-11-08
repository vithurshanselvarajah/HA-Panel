package rpc

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"io"
	"net"
	"os"
	"sync"
)

const Version = "2.0"

const (
	CodeMethodNotFound = -32601

	CodeInvalidParams = -32602

	CodeInternal = -32603

	CodeInvalidState = -32000
)

var (
	ErrorNotFound = errors.New("no such method")

	ErrorInvalidParams = errors.New("the parameters are not valid")

	ErrorInvalidState = errors.New("the server is not in a state for that")
)

type Request struct {
	JSONRPC string          `json:"jsonrpc"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params,omitempty"`
	ID      json.RawMessage `json:"id,omitempty"`
}

func (r Request) IsNotification() bool {
	return len(r.ID) == 0 || string(r.ID) == "null"
}

type Response struct {
	JSONRPC string          `json:"jsonrpc"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *Error          `json:"error,omitempty"`
	ID      json.RawMessage `json:"id,omitempty"`
}

type Error struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

func (e *Error) Error() string {
	return e.Message
}

func NewError(code int, message string) *Error {
	return &Error{Code: code, Message: message}
}

type Handler func(ctx context.Context, params json.RawMessage) (any, error)

type Server struct {
	mu       sync.RWMutex
	handlers map[string]Handler
}

func NewServer() *Server {
	return &Server{handlers: make(map[string]Handler)}
}

func (s *Server) Register(method string, handler Handler) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.handlers[method] = handler
}

func (s *Server) Handle(ctx context.Context, request Request) *Response {
	s.mu.RLock()
	handler, found := s.handlers[request.Method]
	s.mu.RUnlock()

	if !found {
		if request.IsNotification() {
			return nil
		}
		return &Response{
			JSONRPC: Version,
			ID:      request.ID,
			Error:   NewError(CodeMethodNotFound, "no such method: "+request.Method),
		}
	}

	result, err := handler(ctx, request.Params)
	if err != nil {
		if request.IsNotification() {
			return nil
		}

		code := CodeInternal
		var rpcErr *Error
		if errors.As(err, &rpcErr) {
			code = rpcErr.Code
		} else if errors.Is(err, ErrorNotFound) {
			code = CodeMethodNotFound
		} else if errors.Is(err, ErrorInvalidParams) {
			code = CodeInvalidParams
		} else if errors.Is(err, ErrorInvalidState) {
			code = CodeInvalidState
		}

		return &Response{
			JSONRPC: Version,
			ID:      request.ID,
			Error: &Error{
				Code:    code,
				Message: err.Error(),
			},
		}
	}

	if request.IsNotification() {
		return nil
	}

	encoded, err := json.Marshal(result)
	if err != nil {
		return &Response{
			JSONRPC: Version,
			ID:      request.ID,
			Error:   NewError(CodeInternal, "the result cannot be encoded"),
		}
	}

	return &Response{
		JSONRPC: Version,
		ID:      request.ID,
		Result:  encoded,
	}
}

func (s *Server) ServeStdio(ctx context.Context) error {
	return s.serve(ctx, os.Stdin, os.Stdout)
}

func (s *Server) ServeSocket(ctx context.Context, path string) error {
	listener, err := net.Listen("unix", path)
	if err != nil {
		return err
	}
	defer listener.Close()

	go func() {
		<-ctx.Done()
		_ = listener.Close()
	}()

	for {
		conn, err := listener.Accept()
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return err
		}

		go func() {
			defer conn.Close()
			_ = s.serve(ctx, conn, conn)
		}()
	}
}

func (s *Server) serve(ctx context.Context, reader io.Reader, writer io.Writer) error {
	scanner := bufio.NewScanner(reader)

	scanner.Buffer(make([]byte, 0, 64*1024), 8*1024*1024)

	var writeMu sync.Mutex

	for scanner.Scan() {
		if ctx.Err() != nil {
			return nil
		}

		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}

		var request Request
		if err := json.Unmarshal(line, &request); err != nil {

			response := &Response{
				JSONRPC: Version,
				Error:   NewError(CodeInvalidParams, "the request is not valid JSON"),
			}
			writeMu.Lock()
			_ = writeResponse(writer, response)
			writeMu.Unlock()
			continue
		}

		response := s.Handle(ctx, request)
		if response == nil {
			continue
		}

		writeMu.Lock()
		err := writeResponse(writer, response)
		writeMu.Unlock()
		if err != nil {
			return err
		}
	}

	return scanner.Err()
}

func writeResponse(writer io.Writer, response *Response) error {
	encoded, err := json.Marshal(response)
	if err != nil {
		return err
	}
	_, err = writer.Write(append(encoded, '\n'))
	return err
}
