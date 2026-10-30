package api

import (
	"encoding/json"
	"log/slog"
	"net/http"

	grpcclient "github.com/sirabra/fintech-gateway/internal/grpc"
	"github.com/sirabra/fintech-gateway/internal/pb"
)

// Handlers holds dependencies for all HTTP handlers.
type Handlers struct {
	GRPC   *grpcclient.Client
	Logger *slog.Logger
}

// ── Health ───────────────────────────────────────────────────────────────

func (h *Handlers) Healthz(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte("ok"))
}

// ── Quote ────────────────────────────────────────────────────────────────

type quoteReq struct {
	UserID      string `json:"user_id"`
	FromAsset   string `json:"from_asset"`
	ToAsset     string `json:"to_asset"`
	Amount      string `json:"amount"`
	SlippageBps uint32 `json:"slippage_bps"`
}

func (h *Handlers) Quote(w http.ResponseWriter, r *http.Request) {
	var body quoteReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_json", err.Error())
		return
	}
	resp, err := h.GRPC.Quote(r.Context(), &pb.QuoteRequest{
		UserId:      body.UserID,
		FromAsset:   body.FromAsset,
		ToAsset:     body.ToAsset,
		Amount:      body.Amount,
		SlippageBps: body.SlippageBps,
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── ExecuteSwap ──────────────────────────────────────────────────────────

type executeSwapReq struct {
	UserID       string `json:"user_id"`
	QuoteID      string `json:"quote_id"`
	FromAccount  string `json:"from_account"`
	ToAccount    string `json:"to_account"`
	WalletHandle string `json:"wallet_handle"`
	SignatureHex string `json:"signature_hex"`
}

func (h *Handlers) ExecuteSwap(w http.ResponseWriter, r *http.Request) {
	var body executeSwapReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_json", err.Error())
		return
	}
	resp, err := h.GRPC.ExecuteSwap(r.Context(), &pb.ExecuteSwapRequest{
		UserId:       body.UserID,
		QuoteId:      body.QuoteID,
		FromAccount:  body.FromAccount,
		ToAccount:    body.ToAccount,
		WalletHandle: body.WalletHandle,
		Signature:    []byte(body.SignatureHex),
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Status ───────────────────────────────────────────────────────────────

func (h *Handlers) SwapStatus(w http.ResponseWriter, r *http.Request) {
	chain := r.URL.Query().Get("chain")
	txHash := r.URL.Query().Get("tx_hash")
	resp, err := h.GRPC.GetSwapStatus(r.Context(), &pb.StatusRequest{
		Chain:  chain,
		TxHash: txHash,
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Wallet ───────────────────────────────────────────────────────────────

func (h *Handlers) GetWallet(w http.ResponseWriter, r *http.Request) {
	handle := r.PathValue("handle")
	resp, err := h.GRPC.GetWallet(r.Context(), &pb.GetWalletRequest{Handle: handle})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

func (h *Handlers) ListWallets(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	resp, err := h.GRPC.ListWallets(r.Context(), &pb.ListWalletsRequest{UserId: userID})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Portfolio ────────────────────────────────────────────────────────────

func (h *Handlers) GetPortfolio(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	accountID := r.URL.Query().Get("account_id")
	resp, err := h.GRPC.GetPortfolio(r.Context(), &pb.PortfolioRequest{
		UserId:    userID,
		AccountId: accountID,
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Policy ───────────────────────────────────────────────────────────────

func (h *Handlers) GetLimits(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	resp, err := h.GRPC.GetLimits(r.Context(), &pb.LimitsRequest{UserId: userID})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

func (h *Handlers) DailyTotal(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	resp, err := h.GRPC.DailyTotal(r.Context(), &pb.DailyTotalRequest{UserId: userID})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Build ────────────────────────────────────────────────────────────────

type buildReq struct {
	QuoteID     string `json:"quote_id"`
	FromAddress string `json:"from_address"`
}

func (h *Handlers) Build(w http.ResponseWriter, r *http.Request) {
	var body buildReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_json", err.Error())
		return
	}
	resp, err := h.GRPC.Build(r.Context(), &pb.BuildRequest{
		QuoteId:     body.QuoteID,
		FromAddress: body.FromAddress,
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}

// ── Simulate ─────────────────────────────────────────────────────────────

type simulateReq struct {
	UnsignedTxJson string `json:"unsigned_tx_json"`
}

func (h *Handlers) Simulate(w http.ResponseWriter, r *http.Request) {
	var body simulateReq
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, h.Logger, http.StatusBadRequest, "invalid_json", err.Error())
		return
	}
	resp, err := h.GRPC.Simulate(r.Context(), &pb.SimulateRequest{
		UnsignedTxJson: body.UnsignedTxJson,
	})
	if err != nil {
		hs, code := grpcToHTTP(err)
		writeError(w, h.Logger, hs, code, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, resp)
}
