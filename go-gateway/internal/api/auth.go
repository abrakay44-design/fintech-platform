package api

import (
	"encoding/json"
	"net/http"

	"github.com/sirabra/fintech-gateway/internal/auth"
)

type loginReq struct {
	UserID string `json:"user_id"`
}

type loginResp struct {
	Token string `json:"token"`
}

// Login is a mock endpoint for D1. In D2, this will be replaced with WebAuthn.
func (h *Handlers) Login(w http.ResponseWriter, r *http.Request) {
	var body loginReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.UserID == "" {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_request", "user_id is required")
		return
	}

	token, err := auth.GenerateToken(body.UserID)
	if err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "token_error", err.Error())
		return
	}

	writeJSON(w, http.StatusOK, loginResp{Token: token})
}
