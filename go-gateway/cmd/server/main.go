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

	"github.com/sirabra/fintech-gateway/internal/api"
	grpcclient "github.com/sirabra/fintech-gateway/internal/grpc"
)

func main() {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))

	grpcAddr := envOr("GRPC_TARGET", "127.0.0.1:50051")
	httpAddr := envOr("HTTP_ADDR", ":8080")

	gc, err := grpcclient.New(grpcAddr)
	if err != nil {
		logger.Error("failed to dial gRPC server", "err", err, "target", grpcAddr)
		os.Exit(1)
	}
	defer gc.Close()
	logger.Info("connected to gRPC server", "target", grpcAddr)

	router := api.NewRouter(gc, logger)

	srv := &http.Server{
		Addr:              httpAddr,
		Handler:           router,
		ReadHeaderTimeout: 5 * time.Second,
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
