package auth

import (
	"encoding/base64"
	"fmt"
	"sync"
	"time"

	"github.com/go-webauthn/webauthn/protocol"
	"github.com/go-webauthn/webauthn/webauthn"
)

// WebAuthnUser implements webauthn.User for our MVP.
type WebAuthnUser struct {
	ID          string
	Name        string
	DisplayName string
	Credentials []webauthn.Credential
}

func (u *WebAuthnUser) WebAuthnID() []byte                         { return []byte(u.ID) }
func (u *WebAuthnUser) WebAuthnName() string                       { return u.Name }
func (u *WebAuthnUser) WebAuthnDisplayName() string                { return u.DisplayName }
func (u *WebAuthnUser) WebAuthnCredentials() []webauthn.Credential { return u.Credentials }

// NewWebAuthnManager builds the WebAuthn server config.
// In production, use rp origins matching your real domain and store nothing in-memory.
func NewWebAuthnManager() (*webauthn.WebAuthn, error) {
	return webauthn.New(&webauthn.Config{
		RPDisplayName: "Fintech Swap",
		RPID:          "localhost",
		RPOrigins:     []string{"http://localhost:3000", "http://127.0.0.1:3000"},
	})
}

// SessionStore keeps short-lived WebAuthn challenge sessions between
// Begin* and Finish* calls. In-memory + TTL for MVP.
type SessionStore struct {
	mu sync.Mutex
	m  map[string]entry
}

type entry struct {
	data    webauthn.SessionData
	expires time.Time
}

func NewSessionStore() *SessionStore {
	s := &SessionStore{m: make(map[string]entry)}
	go func() {
		for range time.Tick(1 * time.Minute) {
			s.mu.Lock()
			now := time.Now()
			for k, v := range s.m {
				if now.After(v.expires) {
					delete(s.m, k)
				}
			}
			s.mu.Unlock()
		}
	}()
	return s
}

func (s *SessionStore) Put(key string, data webauthn.SessionData) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.m[key] = entry{data: data, expires: time.Now().Add(5 * time.Minute)}
}

func (s *SessionStore) Pop(key string) (webauthn.SessionData, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	v, ok := s.m[key]
	if !ok || time.Now().After(v.expires) {
		delete(s.m, key)
		return webauthn.SessionData{}, false
	}
	delete(s.m, key)
	return v.data, true
}

// EncodeCredentialID makes credential IDs JSON-safe.
func EncodeCredentialID(id []byte) string {
	return base64.RawURLEncoding.EncodeToString(id)
}

// Wire-friendly webauthn.Credential wrapper for DB storage.
type StoredCredential struct {
	ID              []byte
	PublicKey       []byte
	AttestationType string
	AAGUID          []byte
	SignCount       uint32
}

func (s StoredCredential) ToWebAuthn() webauthn.Credential {
	return webauthn.Credential{
		ID:              s.ID,
		PublicKey:       s.PublicKey,
		AttestationType: s.AttestationType,
		Authenticator: webauthn.Authenticator{
			AAGUID:    s.AAGUID,
			SignCount: s.SignCount,
		},
	}
}

func CredentialFromWebAuthn(c webauthn.Credential) StoredCredential {
	return StoredCredential{
		ID:              c.ID,
		PublicKey:       c.PublicKey,
		AttestationType: c.AttestationType,
		AAGUID:          c.Authenticator.AAGUID,
		SignCount:       c.Authenticator.SignCount,
	}
}

// EnsureSessionDataJSON is exposed for debugging.
func EnsureSessionDataJSON(sd webauthn.SessionData) string {
	return fmt.Sprintf("challenge=%x userVerification=%s", sd.Challenge, sd.UserVerification)
}

var _ = protocol.CredentialCreation{}
