// Package grpcclient is a thin, typed wrapper around the generated gRPC
// client. It owns the connection lifecycle and translates gRPC errors into
// HTTP-friendly error kinds.
package grpcclient

import (
	"context"
	"fmt"
	"time"

	"github.com/sirabra/fintech-gateway/internal/pb"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Client wraps the generated SwapServiceClient.
type Client struct {
	conn *grpc.ClientConn
	sc   pb.SwapServiceClient
}

// New dials the Rust gRPC server. Uses plaintext for local dev; swap for
// TLS credentials when moving to staging/prod.
func New(target string) (*Client, error) {
	conn, err := grpc.NewClient(
		target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("dial %s: %w", target, err)
	}
	return &Client{conn: conn, sc: pb.NewSwapServiceClient(conn)}, nil
}

// Close releases the connection.
func (c *Client) Close() error { return c.conn.Close() }

// Quote calls the Quote RPC with a 5s deadline.
func (c *Client) Quote(ctx context.Context, req *pb.QuoteRequest) (*pb.QuoteResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.Quote(ctx, req)
}

// ExecuteSwap calls the ExecuteSwap RPC with a 30s deadline (longer because
// it may touch chain RPCs).
func (c *Client) ExecuteSwap(ctx context.Context, req *pb.ExecuteSwapRequest) (*pb.ExecuteSwapResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.ExecuteSwap(ctx, req)
}

// GetSwapStatus calls the GetSwapStatus RPC.
func (c *Client) GetSwapStatus(ctx context.Context, req *pb.StatusRequest) (*pb.StatusResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.GetSwapStatus(ctx, req)
}

// RegisterWallet calls the RegisterWallet RPC.
func (c *Client) RegisterWallet(ctx context.Context, req *pb.RegisterWalletRequest) (*pb.RegisterWalletResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.RegisterWallet(ctx, req)
}

// GetWallet calls the GetWallet RPC.
func (c *Client) GetWallet(ctx context.Context, req *pb.GetWalletRequest) (*pb.GetWalletResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.GetWallet(ctx, req)
}

// ListWallets calls the ListWallets RPC.
func (c *Client) ListWallets(ctx context.Context, req *pb.ListWalletsRequest) (*pb.ListWalletsResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.ListWallets(ctx, req)
}

// VerifySignature calls the VerifySignature RPC.
func (c *Client) VerifySignature(ctx context.Context, req *pb.VerifySignatureRequest) (*pb.VerifySignatureResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.VerifySignature(ctx, req)
}

// GetPortfolio calls the GetPortfolio RPC.
func (c *Client) GetPortfolio(ctx context.Context, req *pb.PortfolioRequest) (*pb.PortfolioResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.GetPortfolio(ctx, req)
}

// ListTransactions calls the ListTransactions RPC.
func (c *Client) ListTransactions(ctx context.Context, req *pb.TransactionsRequest) (*pb.TransactionsResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.ListTransactions(ctx, req)
}

// GetLimits calls the GetLimits RPC.
func (c *Client) GetLimits(ctx context.Context, req *pb.LimitsRequest) (*pb.LimitsResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.GetLimits(ctx, req)
}

// DailyTotal calls the DailyTotal RPC.
func (c *Client) DailyTotal(ctx context.Context, req *pb.DailyTotalRequest) (*pb.DailyTotalResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.DailyTotal(ctx, req)
}

// Build calls the Build RPC.
func (c *Client) Build(ctx context.Context, req *pb.BuildRequest) (*pb.BuildResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.Build(ctx, req)
}

// Simulate calls the Simulate RPC.
func (c *Client) Simulate(ctx context.Context, req *pb.SimulateRequest) (*pb.SimulateResponse, error) {
	ctx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()
	return c.sc.Simulate(ctx, req)
}
