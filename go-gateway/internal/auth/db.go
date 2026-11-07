package auth

import (
	"context"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Repo struct {
	pool *pgxpool.Pool
}

func NewRepo(pool *pgxpool.Pool) *Repo {
	return &Repo{pool: pool}
}

// UpsertUser returns the user's UUID. Looks up by username; if not found,
// creates a new row with a freshly-generated UUID.
func (r *Repo) UpsertUser(ctx context.Context, username string) (string, error) {
	var existingID string
	err := r.pool.QueryRow(ctx, `SELECT id FROM webauthn_users WHERE username = $1`, username).Scan(&existingID)
	if err == nil {
		return existingID, nil
	}
	if err != pgx.ErrNoRows {
		return "", err
	}
	newID := uuid.NewString()
	_, err = r.pool.Exec(ctx,
		`INSERT INTO webauthn_users (id, username) VALUES ($1, $2)`,
		newID, username,
	)
	return newID, err
}

// LookupUserID returns the UUID for a username, or pgx.ErrNoRows.
func (r *Repo) LookupUserID(ctx context.Context, username string) (string, error) {
	var id string
	err := r.pool.QueryRow(ctx, `SELECT id FROM webauthn_users WHERE username = $1`, username).Scan(&id)
	return id, err
}

func (r *Repo) GetCredentials(ctx context.Context, userID string) ([]StoredCredential, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, public_key, attestation_type, aaguid, sign_count
		FROM webauthn_credentials WHERE user_id = $1
	`, userID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []StoredCredential
	for rows.Next() {
		var c StoredCredential
		var signCount int64
		if err := rows.Scan(&c.ID, &c.PublicKey, &c.AttestationType, &c.AAGUID, &signCount); err != nil {
			return nil, err
		}
		c.SignCount = uint32(signCount)
		out = append(out, c)
	}
	return out, rows.Err()
}

func (r *Repo) SaveCredential(ctx context.Context, userID string, c StoredCredential) error {
	_, err := r.pool.Exec(ctx, `
		INSERT INTO webauthn_credentials (id, user_id, public_key, attestation_type, aaguid, sign_count)
		VALUES ($1, $2, $3, $4, $5, $6)
		ON CONFLICT (id) DO UPDATE SET sign_count = EXCLUDED.sign_count
	`, c.ID, userID, c.PublicKey, c.AttestationType, c.AAGUID, int64(c.SignCount))
	return err
}

func (r *Repo) UpdateSignCount(ctx context.Context, credID []byte, count uint32) error {
	_, err := r.pool.Exec(ctx,
		`UPDATE webauthn_credentials SET sign_count = $1 WHERE id = $2`,
		int64(count), credID,
	)
	return err
}

func (r *Repo) UserExists(ctx context.Context, userID string) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(ctx,
		`SELECT EXISTS(SELECT 1 FROM webauthn_users WHERE id = $1)`, userID,
	).Scan(&exists)
	if err == pgx.ErrNoRows {
		return false, nil
	}
	return exists, err
}
