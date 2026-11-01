package auth

import (
	"time"
	"github.com/golang-jwt/jwt/v5"
)

// In production, this MUST be an environment variable (e.g., 32+ random bytes).
var jwtSecret = []byte("super-secret-development-key-change-me")

type Claims struct {
	UserID string `json:"user_id"`
	jwt.RegisteredClaims
}

// GenerateToken creates a 24-hour JWT for a user.
func GenerateToken(userID string) (string, error) {
	claims := Claims{
		UserID: userID,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(jwtSecret)
}
