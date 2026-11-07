package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/sirabra/fintech-gateway/internal/api"
	"github.com/sirabra/fintech-gateway/internal/auth"
	grpcclient "github.com/sirabra/fintech-gateway/internal/grpc"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))

	grpcTarget := envOr("GRPC_TARGET", "localhost:50051")
	httpAddr := envOr("HTTP_ADDR", ":8080")
	dbURL := envOr("DATABASE_URL", "postgres://fintech:dev_password@localhost:5432/fintech")

	// gRPC client
	gc, err := grpcclient.New(grpcTarget)
	if err != nil {
		logger.Error("failed to connect to grpc", "err", err)
		os.Exit(1)
	}
	defer gc.Close()

	// Postgres pool (for WebAuthn credentials)
	dbPool, err := pgxpool.New(context.Background(), dbURL)
	if err != nil {
		logger.Error("failed to connect to postgres", "err", err)
		os.Exit(1)
	}
	defer dbPool.Close()

	// WebAuthn manager
	wa, err := auth.NewWebAuthnManager()
	if err != nil {
		logger.Error("failed to create webauthn manager", "err", err)
		os.Exit(1)
	}

	// Router
	router := api.NewRouter(gc, logger, dbPool, wa)

	srv := &http.Server{
		Addr:         httpAddr,
		Handler:      router,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 60 * time.Second,
	}

	go func() {
		logger.Info("http listening", "addr", httpAddr)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			logger.Error("http server error", "err", err)
			os.Exit(1)
		}
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop

	logger.Info("shutting down")
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	_ = srv.Shutdown(ctx)
}

func envOr(k, d string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return d
}
