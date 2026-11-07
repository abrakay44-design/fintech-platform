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

type waBeginReq struct {
	Username string `json:"username"`
}

func (h *Handlers) WARegisterBegin(w http.ResponseWriter, r *http.Request) {
	var body waBeginReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.Username == "" {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_request", "username required")
		return
	}

	ctx := r.Context()
	repo := auth.NewRepo(h.DB)
	userID, err := repo.UpsertUser(ctx, body.Username)
	if err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "db_error", err.Error())
		return
	}

	creds, _ := repo.GetCredentials(ctx, userID)
	user := &auth.WebAuthnUser{ID: userID, Name: body.Username, DisplayName: body.Username}
	for _, c := range creds {
		user.Credentials = append(user.Credentials, c.ToWebAuthn())
	}

	creation, session, err := h.WebAuthn.BeginRegistration(user)
	if err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "webauthn_begin", err.Error())
		return
	}
	h.Sessions.Put("reg:"+body.Username, *session)

	writeJSON(w, http.StatusOK, creation)
}

func (h *Handlers) WARegisterFinish(w http.ResponseWriter, r *http.Request) {
	username := r.URL.Query().Get("username")
	if username == "" {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_request", "username required")
		return
	}

	ctx := r.Context()
	repo := auth.NewRepo(h.DB)
	session, ok := h.Sessions.Pop("reg:" + username)
	if !ok {
		writeError(w, h.Logger, http.StatusBadRequest, "no_session", "session expired")
		return
	}

	userID, err := repo.UpsertUser(ctx, username)
	if err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "db_error", err.Error())
		return
	}

	user := &auth.WebAuthnUser{ID: userID, Name: username, DisplayName: username}
	credential, err := h.WebAuthn.FinishRegistration(user, session, r)
	if err != nil {
		writeError(w, h.Logger, http.StatusBadRequest, "webauthn_finish", err.Error())
		return
	}

	if err := repo.SaveCredential(ctx, userID, auth.CredentialFromWebAuthn(*credential)); err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "db_error", err.Error())
		return
	}

	token, _ := auth.GenerateToken(userID)
	writeJSON(w, http.StatusOK, loginResp{Token: token})
}

func (h *Handlers) WALoginBegin(w http.ResponseWriter, r *http.Request) {
	var body waBeginReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.Username == "" {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_request", "username required")
		return
	}

	ctx := r.Context()
	repo := auth.NewRepo(h.DB)
	userID, err := repo.LookupUserID(ctx, body.Username)
	if err != nil {
		writeError(w, h.Logger, http.StatusNotFound, "not_registered", "user not registered")
		return
	}

	creds, _ := repo.GetCredentials(ctx, userID)
	if len(creds) == 0 {
		writeError(w, h.Logger, http.StatusNotFound, "no_credentials", "no passkeys registered")
		return
	}

	user := &auth.WebAuthnUser{ID: userID, Name: body.Username, DisplayName: body.Username}
	for _, c := range creds {
		user.Credentials = append(user.Credentials, c.ToWebAuthn())
	}

	assertion, session, err := h.WebAuthn.BeginLogin(user)
	if err != nil {
		writeError(w, h.Logger, http.StatusInternalServerError, "webauthn_begin", err.Error())
		return
	}
	h.Sessions.Put("login:"+body.Username, *session)

	writeJSON(w, http.StatusOK, assertion)
}

func (h *Handlers) WALoginFinish(w http.ResponseWriter, r *http.Request) {
	username := r.URL.Query().Get("username")
	if username == "" {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_request", "username required")
		return
	}

	ctx := r.Context()
	repo := auth.NewRepo(h.DB)
	session, ok := h.Sessions.Pop("login:" + username)
	if !ok {
		writeError(w, h.Logger, http.StatusBadRequest, "no_session", "session expired")
		return
	}

	userID, err := repo.LookupUserID(ctx, username)
	if err != nil {
		writeError(w, h.Logger, http.StatusNotFound, "not_registered", "user not registered")
		return
	}

	creds, _ := repo.GetCredentials(ctx, userID)
	user := &auth.WebAuthnUser{ID: userID, Name: username, DisplayName: username}
	for _, c := range creds {
		user.Credentials = append(user.Credentials, c.ToWebAuthn())
	}

	credential, err := h.WebAuthn.FinishLogin(user, session, r)
	if err != nil {
		writeError(w, h.Logger, http.StatusUnauthorized, "webauthn_finish", err.Error())
		return
	}

	_ = repo.UpdateSignCount(ctx, credential.ID, credential.Authenticator.SignCount)

	token, _ := auth.GenerateToken(userID)
	writeJSON(w, http.StatusOK, loginResp{Token: token})
}
