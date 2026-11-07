"use client";

import { useEffect, useState } from "react";
import axios from "axios";
import { bufferToBase64Url, base64UrlToBuffer } from "@/lib/webauthn";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const [authToken, setAuthToken] = useState<string | null>(null);
  const [username, setUsername] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Swap state
  const [amount, setAmount] = useState("0.001");
  const [swapResult, setSwapResult] = useState<any>(null);

  useEffect(() => { setMounted(true); }, []);

  async function handleRegister() {
    if (!username) return setStatus("Enter an email first.");
    setLoading(true);
    setStatus("Waiting for biometrics (register)...");

    try {
      // 1. Get challenge
      const beginRes = await axios.post(`${API_URL}/auth/webauthn/register/begin`, { username });
      const options = beginRes.data.publicKey;

      // 2. Convert challenge and user.id to ArrayBuffer
      options.challenge = base64UrlToBuffer(options.challenge);
      options.user.id = base64UrlToBuffer(options.user.id);

      // 3. Trigger biometric prompt
      const credential = (await navigator.credentials.create({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("Registration cancelled");

      const response = credential.response as AuthenticatorAttestationResponse;

      // 4. Send result to backend
      const finishRes = await axios.post(`${API_URL}/auth/webauthn/register/finish?username=${encodeURIComponent(username)}`, {
        id: credential.id,
        rawId: bufferToBase64Url(credential.rawId),
        type: credential.type,
        response: {
          clientDataJSON: bufferToBase64Url(response.clientDataJSON),
          attestationObject: bufferToBase64Url(response.attestationObject),
        },
      });

      setAuthToken(finishRes.data.token);
      setStatus("✅ Passkey registered and logged in!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.error || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleLogin() {
    if (!username) return setStatus("Enter an email first.");
    setLoading(true);
    setStatus("Waiting for biometrics (login)...");

    try {
      // 1. Get challenge
      const beginRes = await axios.post(`${API_URL}/auth/webauthn/login/begin`, { username });
      const options = beginRes.data.publicKey;

      // 2. Convert challenge and allowCredentials to ArrayBuffer
      options.challenge = base64UrlToBuffer(options.challenge);
      if (options.allowCredentials) {
        options.allowCredentials = options.allowCredentials.map((c: any) => ({
          ...c,
          id: base64UrlToBuffer(c.id),
        }));
      }

      // 3. Trigger biometric prompt
      const credential = (await navigator.credentials.get({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("Login cancelled");

      const response = credential.response as AuthenticatorAssertionResponse;

      // 4. Send result to backend
      const finishRes = await axios.post(`${API_URL}/auth/webauthn/login/finish?username=${encodeURIComponent(username)}`, {
        id: credential.id,
        rawId: bufferToBase64Url(credential.rawId),
        type: credential.type,
        response: {
          clientDataJSON: bufferToBase64Url(response.clientDataJSON),
          authenticatorData: bufferToBase64Url(response.authenticatorData),
          signature: bufferToBase64Url(response.signature),
          userHandle: response.userHandle ? bufferToBase64Url(response.userHandle) : null,
        },
      });

      setAuthToken(finishRes.data.token);
      setStatus("✅ Logged in successfully!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.error || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleSwap() {
    if (!authToken) return;
    setLoading(true);
    setStatus("1. Fetching quote...");
    setSwapResult(null);

    try {
      const config = { headers: { Authorization: `Bearer ${authToken}` } };

      const quoteRes = await axios.post(`${API_URL}/v1/transactions/quote`, {
        from_asset: "ETH", to_asset: "USDC", amount, slippage_bps: 50,
      }, config);
      const quote = quoteRes.data;

      setStatus("2. Building transaction...");
      const buildRes = await axios.post(`${API_URL}/v1/transactions/build`, {
        quote_id: quote.quote_id, from_address: "0x0000000000000000000000000000000000000001",
      }, config);

      setStatus("3. Simulating on Sepolia...");
      const simRes = await axios.post(`${API_URL}/v1/transactions/simulate`, {
        unsigned_tx_json: buildRes.data.unsigned_tx_json,
      }, config);

      setSwapResult({ quote, simulation: simRes.data });
      setStatus("Success!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.error || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  if (!mounted) return null;

  return (
    <main className="min-h-screen bg-gray-950 text-white flex flex-col items-center justify-center p-4">
      <div className="w-full max-w-md bg-gray-900 rounded-2xl p-6 shadow-xl border border-gray-800">
        <h1 className="text-2xl font-bold mb-6">Fintech Passkeys</h1>

        {!authToken ? (
          <div className="space-y-4">
            <div className="bg-gray-800 p-4 rounded-xl">
              <label className="text-sm text-gray-400 mb-1 block">Email / Username</label>
              <input
                type="text"
                placeholder="you@example.com"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full bg-transparent text-lg focus:outline-none"
              />
            </div>
            
            <div className="flex gap-2">
              <button
                onClick={handleRegister}
                disabled={loading}
                className="flex-1 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 rounded-xl font-bold transition"
              >
                Register
              </button>
              <button
                onClick={handleLogin}
                disabled={loading}
                className="flex-1 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 rounded-xl font-bold transition"
              >
                Login
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex justify-between items-center mb-4">
              <span className="text-green-400 text-sm">● Authenticated</span>
              <button onClick={() => setAuthToken(null)} className="text-xs text-gray-400 hover:text-white underline">
                Log out
              </button>
            </div>

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
              disabled={loading}
              className="w-full py-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 rounded-xl font-bold text-lg transition"
            >
              {loading ? "Processing..." : "Swap ETH for USDC"}
            </button>

            {swapResult && (
              <div className="mt-4 p-4 bg-black rounded-xl border border-gray-700 overflow-x-auto text-xs font-mono text-green-400">
                <p className="mb-2">Quote: {swapResult.quote.expected_out} USDC</p>
                <p>Simulation: {swapResult.simulation.success ? "✅ Success" : "❌ Failed"}</p>
              </div>
            )}
          </div>
        )}

        {status && <p className="mt-4 text-center text-sm font-mono text-blue-400 break-words">{status}</p>}
      </div>
    </main>
  );
}
