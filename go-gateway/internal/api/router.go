package api

import (
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
	"github.com/go-webauthn/webauthn/webauthn"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/sirabra/fintech-gateway/internal/auth"
	grpcclient "github.com/sirabra/fintech-gateway/internal/grpc"
)

func NewRouter(gc *grpcclient.Client, logger *slog.Logger, db *pgxpool.Pool, wa *webauthn.WebAuthn) http.Handler {
	h := &Handlers{
		GRPC:     gc,
		Logger:   logger,
		DB:       db,
		WebAuthn: wa,
		Sessions: auth.NewSessionStore(),
	}

	r := chi.NewRouter()
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Recoverer)
	r.Use(middleware.Timeout(60 * time.Second))
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"http://localhost:3000", "http://127.0.0.1:3000"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type", "X-CSRF-Token"},
		AllowCredentials: true,
		MaxAge:           300,
	}))

	r.Get("/healthz", h.Healthz)
	r.Post("/auth/login", h.Login)
	r.Post("/auth/webauthn/register/begin", h.WARegisterBegin)
	r.Post("/auth/webauthn/register/finish", h.WARegisterFinish)
	r.Post("/auth/webauthn/login/begin", h.WALoginBegin)
	r.Post("/auth/webauthn/login/finish", h.WALoginFinish)

	r.Route("/v1", func(r chi.Router) {
		r.Use(auth.RequireAuth)
		r.Route("/transactions", func(r chi.Router) {
			r.Post("/quote", h.Quote)
			r.Post("/build", h.Build)
			r.Post("/simulate", h.Simulate)
			r.Post("/broadcast", h.BroadcastTransaction)
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
