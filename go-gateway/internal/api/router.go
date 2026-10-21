package api

import (
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	grpcclient "github.com/sirabra/fintech-gateway/internal/grpc"
)

// NewRouter builds the chi router with all REST endpoints wired to the
// gRPC client.
func NewRouter(gc *grpcclient.Client, logger *slog.Logger) http.Handler {
	h := &Handlers{GRPC: gc, Logger: logger}

	r := chi.NewRouter()
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Recoverer)
	r.Use(middleware.Timeout(60 * time.Second))

	r.Get("/healthz", h.Healthz)

	r.Route("/v1", func(r chi.Router) {
		r.Route("/transactions", func(r chi.Router) {
			r.Post("/quote", h.Quote)
			r.Post("/execute", h.ExecuteSwap)
			r.Get("/status", h.SwapStatus)
		})

		r.Route("/wallet", func(r chi.Router) {
			r.Get("/{handle}", h.GetWallet)
			r.Get("/", h.ListWallets)
		})

		r.Route("/portfolio", func(r chi.Router) {
			r.Get("/", h.GetPortfolio)
		})

		r.Route("/policy", func(r chi.Router) {
			r.Get("/limits", h.GetLimits)
			r.Get("/daily-total", h.DailyTotal)
		})
	})

	return r
}
