"use client";

import { useEffect, useState } from "react";
import axios from "axios";
import { useAccount, useConnect, useDisconnect, useWalletClient } from "wagmi";
import { injected } from "wagmi/connectors";
import { bufferToBase64Url, base64UrlToBuffer } from "@/lib/webauthn";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const [authToken, setAuthToken] = useState<string | null>(null);
  const [username, setUsername] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [amount, setAmount] = useState("0.001");
  const [swapResult, setSwapResult] = useState<any>(null);
  const [broadcastStatus, setBroadcastStatus] = useState<string | null>(null);
  const [txHash, setTxHash] = useState<string | null>(null);

  const { address, isConnected } = useAccount();
  const { connect, connectors } = useConnect();
  const { disconnect } = useDisconnect();
  const { data: walletClient } = useWalletClient();

  useEffect(() => { setMounted(true); }, []);

  async function handleRegister() {
    if (!username) return setStatus("Enter an email first.");
    setLoading(true);
    setStatus("Waiting for biometrics (register)...");
    try {
      const beginRes = await axios.post(`${API_URL}/auth/webauthn/register/begin`, { username });
      const options = beginRes.data.publicKey;
      options.challenge = base64UrlToBuffer(options.challenge);
      options.user.id = base64UrlToBuffer(options.user.id);

      const credential = (await navigator.credentials.create({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("Registration cancelled");
      const response = credential.response as AuthenticatorAttestationResponse;

      const finishRes = await axios.post(
        `${API_URL}/auth/webauthn/register/finish?username=${encodeURIComponent(username)}`,
        {
          id: credential.id,
          rawId: bufferToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
            attestationObject: bufferToBase64Url(response.attestationObject),
          },
        }
      );
      setAuthToken(finishRes.data.token);
      setStatus("Passkey registered and logged in!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleLogin() {
    if (!username) return setStatus("Enter an email first.");
    setLoading(true);
    setStatus("Waiting for biometrics (login)...");
    try {
      const beginRes = await axios.post(`${API_URL}/auth/webauthn/login/begin`, { username });
      const options = beginRes.data.publicKey;
      options.challenge = base64UrlToBuffer(options.challenge);
      if (options.allowCredentials) {
        options.allowCredentials = options.allowCredentials.map((c: any) => ({
          ...c, id: base64UrlToBuffer(c.id),
        }));
      }
      const credential = (await navigator.credentials.get({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("Login cancelled");
      const response = credential.response as AuthenticatorAssertionResponse;

      const finishRes = await axios.post(
        `${API_URL}/auth/webauthn/login/finish?username=${encodeURIComponent(username)}`,
        {
          id: credential.id,
          rawId: bufferToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
            authenticatorData: bufferToBase64Url(response.authenticatorData),
            signature: bufferToBase64Url(response.signature),
            userHandle: response.userHandle ? bufferToBase64Url(response.userHandle) : null,
          },
        }
      );
      setAuthToken(finishRes.data.token);
      setStatus("Logged in successfully!");
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleSwap() {
    if (!authToken) return;
    if (!isConnected || !address) {
      setStatus("Please connect your MetaMask wallet first.");
      return;
    }

    setLoading(true);
    setStatus("1. Fetching quote...");
    setSwapResult(null);
    setBroadcastStatus(null);
    setTxHash(null);

    try {
      const config = { headers: { Authorization: `Bearer ${authToken}` } };

      const quoteRes = await axios.post(`${API_URL}/v1/transactions/quote`, {
        from_asset: "ETH", to_asset: "USDC", amount, slippage_bps: 50,
      }, config);
      const quote = quoteRes.data;

      setStatus("2. Building transaction...");
      const buildRes = await axios.post(`${API_URL}/v1/transactions/build`, {
        quote_id: quote.quote_id,
        from_address: address,
      }, config);
      const unsignedTxJson = buildRes.data.unsigned_tx_json;

      setStatus("3. Simulating on Sepolia...");
      const simRes = await axios.post(`${API_URL}/v1/transactions/simulate`, {
        unsigned_tx_json: unsignedTxJson,
      }, config);

      setSwapResult({ quote, unsignedTxJson, simulation: simRes.data });
      if (simRes.data.success) {
        setStatus("Success! Ready to broadcast.");
      } else {
        setStatus(`Simulation failed: ${simRes.data.revert_reason || "unknown reason"}`);
      }
    } catch (err: any) {
      console.error(err);
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleBroadcast() {
    if (!authToken || !swapResult || !isConnected) return;
    if (!walletClient) {
      setBroadcastStatus("Wallet client not ready. Try reconnecting.");
      return;
    }

    setLoading(true);
    setBroadcastStatus("Requesting signature from wallet...");

    try {
      const unsignedTx = JSON.parse(swapResult.unsignedTxJson);

      // MetaMask does not support eth_signTransaction.
      // Use sendTransaction instead: wallet signs AND broadcasts in one step.
      const hash = await walletClient.sendTransaction({
        to: unsignedTx.to,
        value: BigInt(unsignedTx.value || "0x0"),
        gas: BigInt(unsignedTx.gas || "0x186a0"),
        maxFeePerGas: BigInt(unsignedTx.maxFeePerGas),
        maxPriorityFeePerGas: BigInt(unsignedTx.maxPriorityFeePerGas),
        nonce: Number(unsignedTx.nonce),
        chainId: Number(unsignedTx.chainId),
      });

      setTxHash(hash);
      setBroadcastStatus("Broadcast! Waiting for confirmation...");
    } catch (err: any) {
      console.error(err);
      setBroadcastStatus(`Error: ${err.message}`);
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
                type="text" placeholder="you@example.com" value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full bg-transparent text-lg focus:outline-none"
              />
            </div>
            <div className="flex gap-2">
              <button onClick={handleRegister} disabled={loading}
                className="flex-1 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 rounded-xl font-bold transition">
                Register Passkey
              </button>
              <button onClick={handleLogin} disabled={loading}
                className="flex-1 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 rounded-xl font-bold transition">
                Login
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex justify-between items-center mb-4">
              <span className="text-green-400 text-sm">Authenticated</span>
              <button onClick={() => { setAuthToken(null); setSwapResult(null); setTxHash(null); setBroadcastStatus(null); setStatus(null); }}
                className="text-xs text-gray-400 hover:text-white underline">
                Log out
              </button>
            </div>

            {!isConnected ? (
              <button
                onClick={() => connect({ connector: connectors[0] })}
                className="w-full py-3 bg-orange-600 hover:bg-orange-700 rounded-xl font-bold transition"
              >
                Connect MetaMask Wallet
              </button>
            ) : (
              <div className="flex justify-between items-center bg-gray-800 p-3 rounded-xl">
                <span className="text-sm text-gray-300 font-mono">
                  {address?.slice(0, 6)}...{address?.slice(-4)}
                </span>
                <button onClick={() => disconnect()} className="text-xs text-red-400 hover:text-red-300 underline">
                  Disconnect
                </button>
              </div>
            )}

            <div className="bg-gray-800 p-4 rounded-xl">
              <label className="text-sm text-gray-400 mb-1 block">Amount (ETH)</label>
              <input type="number" value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full bg-transparent text-2xl font-mono focus:outline-none" />
            </div>

            <button onClick={handleSwap} disabled={loading || !isConnected}
              className="w-full py-4 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 rounded-xl font-bold text-lg transition">
              {loading && !broadcastStatus ? "Processing..." : !isConnected ? "Connect Wallet First" : "Swap ETH for USDC"}
            </button>

            {swapResult && (
              <div className="mt-4 p-4 bg-black rounded-xl border border-gray-700 text-xs font-mono text-green-400">
                <p className="mb-2">Quote: {swapResult.quote.expected_out} USDC</p>
                <p className="mb-2">Simulation: {swapResult.simulation.success ? "Success" : "Failed"}</p>
              </div>
            )}

            {swapResult && swapResult.simulation.success && !txHash && (
              <button onClick={handleBroadcast} disabled={loading}
                className="w-full py-3 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-700 rounded-xl font-bold transition">
                {broadcastStatus?.includes("signature") || broadcastStatus?.includes("Broadcasting")
                  ? broadcastStatus
                  : "Sign & Broadcast"}
              </button>
            )}

            {broadcastStatus && !txHash && (
              <p className="text-center text-sm font-mono text-purple-400">{broadcastStatus}</p>
            )}

            {txHash && (
              <div className="mt-2 p-4 bg-black rounded-xl border border-green-700 text-xs font-mono">
                <p className="text-green-400 mb-2">Transaction broadcast!</p>
                <a href={`https://sepolia.etherscan.io/tx/${txHash}`}
                  target="_blank" rel="noopener noreferrer"
                  className="text-blue-400 hover:underline break-all">
                  {txHash}
                </a>
              </div>
            )}
          </div>
        )}

        {status && <p className="mt-4 text-center text-sm font-mono text-blue-400 break-words">{status}</p>}
      </div>
    </main>
  );
}
