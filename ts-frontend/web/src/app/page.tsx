"use client";

import { useEffect, useState } from "react";
import { useAccount, useConnect, useDisconnect } from "wagmi";
import axios from "axios";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const [authToken, setAuthToken] = useState<string | null>(null);

  useEffect(() => { setMounted(true); }, []);

  const { address, isConnected } = useAccount();
  const { connect, connectors } = useConnect();
  const { disconnect } = useDisconnect();

  const [amount, setAmount] = useState("0.001");
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  // Mock login when wallet connects
  useEffect(() => {
    async function login() {
      if (!isConnected || !address) return;
      try {
        const res = await axios.post(`${API_URL}/auth/login`, {
          user_id: "12345678-1234-1234-1234-1234567890ab" // Mock user ID mapped to this wallet
        });
        setAuthToken(res.data.token);
        setStatus("Authenticated successfully");
      } catch (err) {
        setStatus("Failed to authenticate");
      }
    }
    login();
  }, [isConnected, address]);

  async function handleSwap() {
    if (!isConnected || !address || !authToken) return;
    setLoading(true);
    setStatus("1. Fetching quote...");
    setResult(null);

    try {
      const config = { headers: { Authorization: `Bearer ${authToken}` } };

      const quoteRes = await axios.post(`${API_URL}/v1/transactions/quote`, {
        from_asset: "ETH",
        to_asset: "USDC",
        amount,
        slippage_bps: 50,
      }, config);
      const quote = quoteRes.data;

      setStatus("2. Building transaction...");
      const buildRes = await axios.post(`${API_URL}/v1/transactions/build`, {
        quote_id: quote.quote_id,
        from_address: address,
      }, config);
      const unsignedTx = buildRes.data.unsigned_tx_json;

      setStatus("3. Simulating on Sepolia...");
      const simRes = await axios.post(`${API_URL}/v1/transactions/simulate`, {
        unsigned_tx_json: unsignedTx,
      }, config);

      setResult({ quote, unsignedTx, simulation: simRes.data });
      setStatus("Success!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.error || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="min-h-screen bg-gray-950 text-white flex flex-col items-center justify-center p-4">
      <div className="w-full max-w-md bg-gray-900 rounded-2xl p-6 shadow-xl border border-gray-800">
        <div className="flex justify-between items-center mb-6">
          <h1 className="text-2xl font-bold">Fintech Swap</h1>
          {!mounted ? (
            <div className="px-4 py-2 w-32 h-9 bg-gray-800 rounded-lg animate-pulse" />
          ) : isConnected ? (
            <button
              onClick={() => { disconnect(); setAuthToken(null); setStatus(null); setResult(null); }}
              className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg text-sm font-medium transition"
            >
              Disconnect
            </button>
          ) : (
            <button
              onClick={() => connect({ connector: connectors[0] })}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition"
            >
              Connect Wallet
            </button>
          )}
        </div>

        {mounted && isConnected && (
          <div className="space-y-4">
            <div className="bg-gray-800 p-4 rounded-xl">
              <label className="text-sm text-gray-400 mb-1 block">Amount (ETH)</label>
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full bg-transparent text-2xl font-mono focus:outline-none"
              />
            </div>

            <button
              onClick={handleSwap}
              disabled={loading || !authToken}
              className="w-full py-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 rounded-xl font-bold text-lg transition"
            >
              {loading ? "Processing..." : !authToken ? "Authenticating..." : "Swap ETH for USDC"}
            </button>

            {status && <p className="text-center text-sm font-mono text-blue-400">{status}</p>}
            {result && (
              <div className="mt-4 p-4 bg-black rounded-xl border border-gray-700 overflow-x-auto text-xs font-mono text-green-400">
                <p className="mb-2">Quote: {result.quote.expected_out} USDC</p>
                <p className="mb-2">Simulation: {result.simulation.success ? "✅ Success" : "❌ Failed"}</p>
              </div>
            )}
          </div>
        )}
      </div>
    </main>
  );
}
