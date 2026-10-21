package api

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// errorResponse is the JSON body returned on any error.
type errorResponse struct {
	Error   string `json:"error"`
	Code    string `json:"code"`
	Message string `json:"message,omitempty"`
}

// writeError logs and returns a JSON error.
func writeError(w http.ResponseWriter, logger *slog.Logger, httpStatus int, code, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(httpStatus)
	_ = json.NewEncoder(w).Encode(errorResponse{
		Error: code,
		Code:  code,
		Message: msg,
	})
}

// writeJSON serializes v as JSON with the given status.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// grpcToHTTP maps gRPC status codes to HTTP status codes.
func grpcToHTTP(err error) (int, string) {
	st, ok := status.FromError(err)
	if !ok {
		return http.StatusInternalServerError, "internal"
	}
	switch st.Code() {
	case codes.InvalidArgument:
		return http.StatusBadRequest, "invalid_argument"
	case codes.NotFound:
		return http.StatusNotFound, "not_found"
	case codes.Unauthenticated:
		return http.StatusUnauthorized, "unauthenticated"
	case codes.PermissionDenied:
		return http.StatusForbidden, "permission_denied"
	case codes.FailedPrecondition:
		return http.StatusConflict, "failed_precondition"
	case codes.DeadlineExceeded:
		return http.StatusGatewayTimeout, "deadline_exceeded"
	case codes.Unavailable:
		return http.StatusServiceUnavailable, "unavailable"
	default:
		return http.StatusInternalServerError, "internal"
	}
}
