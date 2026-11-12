"use client";

import { useEffect, useState } from "react";
import axios from "axios";
import { useAccount, useConnect, useDisconnect, useWalletClient } from "wagmi";
import { bufferToBase64Url, base64UrlToBuffer } from "@/lib/webauthn";
import { ShieldCheck, Wallet, ArrowDown, ExternalLink, Activity, ChevronDown, Info } from "lucide-react";

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
  
  // UI state for neutral routing details
  const [showDetails, setShowDetails] = useState(false);

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
          id: credential.id, rawId: bufferToBase64Url(credential.rawId), type: credential.type,
          response: { clientDataJSON: bufferToBase64Url(response.clientDataJSON), attestationObject: bufferToBase64Url(response.attestationObject) },
        }
      );
      setAuthToken(finishRes.data.token);
      setStatus("Passkey registered successfully.");
    } catch (err: any) {
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally { setLoading(false); }
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
        options.allowCredentials = options.allowCredentials.map((c: any) => ({ ...c, id: base64UrlToBuffer(c.id) }));
      }
      const credential = (await navigator.credentials.get({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("Login cancelled");
      const response = credential.response as AuthenticatorAssertionResponse;
      const finishRes = await axios.post(
        `${API_URL}/auth/webauthn/login/finish?username=${encodeURIComponent(username)}`,
        {
          id: credential.id, rawId: bufferToBase64Url(credential.rawId), type: credential.type,
          response: {
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
            authenticatorData: bufferToBase64Url(response.authenticatorData),
            signature: bufferToBase64Url(response.signature),
            userHandle: response.userHandle ? bufferToBase64Url(response.userHandle) : null,
          },
        }
      );
      setAuthToken(finishRes.data.token);
      setStatus("Logged in successfully.");
    } catch (err: any) {
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally { setLoading(false); }
  }

  async function handleSwap() {
    if (!authToken || !isConnected || !address) return;
    setLoading(true);
    setStatus("1. Fetching quote...");
    setSwapResult(null);
    setBroadcastStatus(null);
    setTxHash(null);
    setShowDetails(false);
    
    try {
      const config = { headers: { Authorization: `Bearer ${authToken}` } };
      const quoteRes = await axios.post(`${API_URL}/v1/transactions/quote`, { from_asset: "ETH", to_asset: "USDC", amount, slippage_bps: 50 }, config);
      const quote = quoteRes.data;
      
      setStatus("2. Building transaction...");
      const buildRes = await axios.post(`${API_URL}/v1/transactions/build`, { quote_id: quote.quote_id, from_address: address }, config);
      const unsignedTxJson = buildRes.data.unsigned_tx_json;
      
      setStatus("3. Simulating on Sepolia...");
      const simRes = await axios.post(`${API_URL}/v1/transactions/simulate`, { unsigned_tx_json: unsignedTxJson }, config);
      
      setSwapResult({ quote, unsignedTxJson, simulation: simRes.data });
      if (simRes.data.success) setStatus("Simulation successful. Review details below.");
      else setStatus(`Simulation failed: ${simRes.data.revert_reason || "unknown"}`);
    } catch (err: any) {
      setStatus(`Error: ${err.response?.data?.message || err.message}`);
    } finally { setLoading(false); }
  }

  async function handleBroadcast() {
    if (!authToken || !swapResult || !isConnected || !walletClient) return;
    setLoading(true);
    setBroadcastStatus("Requesting signature...");
    try {
      const unsignedTx = JSON.parse(swapResult.unsignedTxJson);
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
      setBroadcastStatus("Broadcast successful!");
    } catch (err: any) {
      setBroadcastStatus(`Error: ${err.message}`);
    } finally { setLoading(false); }
  }

  if (!mounted) return null;

  return (
    <div className="min-h-screen bg-[#0B0E14] text-slate-200 font-sans selection:bg-indigo-500/30 flex flex-col">
      <nav className="border-b border-white/5 bg-[#0B0E14]/80 backdrop-blur-md sticky top-0 z-50">
        <div className="max-w-6xl mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center">
              <Activity size={18} className="text-white" />
            </div>
            <span className="font-bold text-lg tracking-tight text-white">Fintech<span className="text-indigo-400">Pass</span></span>
          </div>
          <div className="flex items-center gap-4">
            {authToken && (
              <div className="hidden sm:flex items-center gap-1.5 text-xs font-medium text-emerald-400 bg-emerald-400/10 px-3 py-1.5 rounded-full">
                <ShieldCheck size={14} /> Passkey Active
              </div>
            )}
            {isConnected ? (
              <button onClick={() => disconnect()} className="text-sm font-medium text-slate-400 hover:text-white transition">
                {address?.slice(0, 6)}...{address?.slice(-4)}
              </button>
            ) : (
              <button onClick={() => connect({ connector: connectors[0] })} className="flex items-center gap-2 text-sm font-semibold bg-white text-black px-4 py-2 rounded-xl hover:bg-slate-200 transition">
                <Wallet size={16} /> Connect Wallet
              </button>
            )}
          </div>
        </div>
      </nav>

      <main className="flex-1 max-w-lg mx-auto w-full px-4 pt-12 pb-12">
        {!authToken ? (
          <div className="bg-[#131722] border border-white/5 rounded-3xl p-8 shadow-2xl">
            <div className="text-center mb-8">
              <h1 className="text-2xl font-bold text-white mb-2">Welcome Back</h1>
              <p className="text-sm text-slate-400">Authenticate with biometrics to access your dashboard.</p>
            </div>
            <div className="space-y-4">
              <div>
                <label className="text-xs font-medium text-slate-400 mb-1.5 block uppercase tracking-wider">Email Address</label>
                <input type="email" placeholder="you@example.com" value={username} onChange={(e) => setUsername(e.target.value)}
                  className="w-full bg-[#0B0E14] border border-white/10 rounded-xl px-4 py-3.5 text-white placeholder-slate-600 focus:outline-none focus:border-indigo-500 transition" />
              </div>
              <div className="flex gap-3 pt-2">
                <button onClick={handleRegister} disabled={loading} className="flex-1 py-3.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-xl font-semibold transition shadow-lg shadow-indigo-500/20">Register</button>
                <button onClick={handleLogin} disabled={loading} className="flex-1 py-3.5 bg-white/5 hover:bg-white/10 disabled:opacity-50 text-white rounded-xl font-semibold transition border border-white/10">Log In</button>
              </div>
            </div>
          </div>
        ) : (
          <div className="bg-[#131722] border border-white/5 rounded-3xl p-2 shadow-2xl">
            <div className="p-4 flex justify-between items-center">
              <h2 className="font-semibold text-white">Swap</h2>
              <button onClick={() => { setAuthToken(null); setSwapResult(null); setTxHash(null); setStatus(null); }} className="text-xs font-medium text-slate-400 hover:text-white transition">Log out</button>
            </div>
            
            <div className="space-y-1">
              <div className="bg-[#0B0E14] p-4 rounded-2xl border border-white/5">
                <div className="flex justify-between mb-2"><span className="text-sm font-medium text-slate-400">You Pay</span></div>
                <div className="flex items-center justify-between gap-4">
                  <input type="number" value={amount} onChange={(e) => setAmount(e.target.value)} className="w-full bg-transparent text-4xl font-medium text-white placeholder-slate-600 focus:outline-none" placeholder="0.0" />
                  <div className="flex items-center gap-2 bg-white/5 px-3 py-1.5 rounded-lg border border-white/5 shrink-0">
                    <div className="w-6 h-6 rounded-full bg-indigo-500 flex items-center justify-center text-[10px] font-bold">E</div>
                    <span className="font-semibold text-white">ETH</span>
                  </div>
                </div>
              </div>
              
              <div className="flex justify-center -my-2 relative z-10">
                <div className="bg-[#131722] p-2 rounded-xl border border-white/10"><ArrowDown size={16} className="text-slate-400" /></div>
              </div>
              
              <div className="bg-[#0B0E14] p-4 rounded-2xl border border-white/5">
                <div className="flex justify-between mb-2"><span className="text-sm font-medium text-slate-400">You Receive (Est.)</span></div>
                <div className="flex items-center justify-between gap-4">
                  <div className="w-full text-4xl font-medium text-slate-500">{swapResult ? swapResult.quote.expected_out : "0.0"}</div>
                  <div className="flex items-center gap-2 bg-white/5 px-3 py-1.5 rounded-lg border border-white/5 shrink-0">
                    <div className="w-6 h-6 rounded-full bg-blue-500 flex items-center justify-center text-[10px] font-bold">U</div>
                    <span className="font-semibold text-white">USDC</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="p-4 space-y-3">
              <button onClick={handleSwap} disabled={loading || !isConnected} className="w-full py-4 bg-indigo-600 hover:bg-indigo-500 disabled:bg-slate-800 disabled:text-slate-500 text-white rounded-2xl font-bold text-lg transition shadow-lg shadow-indigo-500/20">
                {loading && !broadcastStatus ? "Processing..." : !isConnected ? "Connect Wallet First" : "Review Swap"}
              </button>

              {swapResult && (
                <div className="bg-[#0B0E14] rounded-xl p-3 border border-white/5">
                  {/* Neutral Routing Details Toggle */}
                  <button onClick={() => setShowDetails(!showDetails)} className="w-full flex items-center justify-between text-xs font-medium text-slate-400 hover:text-white transition py-1">
                    <span className="flex items-center gap-1"><Info size={14} /> Transaction Details</span>
                    <ChevronDown size={14} className={`transition-transform ${showDetails ? "rotate-180" : ""}`} />
                  </button>
                  
                  {showDetails && (
                    <div className="mt-3 pt-3 border-t border-white/5 space-y-2 text-xs font-mono">
                      <div className="flex justify-between text-slate-400">
                        <span>Route</span>
                        <span className="text-slate-300">{swapResult.quote.route_summary || "Direct Transfer"}</span>
                      </div>
                      <div className="flex justify-between text-slate-400">
                        <span>Minimum Received</span>
                        <span className="text-slate-300">{swapResult.quote.minimum_out} USDC</span>
                      </div>
                      <div className="flex justify-between text-slate-400">
                        <span>Network Fee (Est.)</span>
                        <span className="text-slate-300">${swapResult.quote.estimated_fee || "0.00"}</span>
                      </div>
                      <div className="flex justify-between text-slate-400">
                        <span>Slippage Tolerance</span>
                        <span className="text-slate-300">0.50%</span>
                      </div>
                    </div>
                  )}

                  <div className="mt-2 flex justify-between text-xs">
                    <span className="text-slate-500">Simulation</span>
                    <span className={swapResult.simulation.success ? "text-emerald-400 font-medium" : "text-red-400 font-medium"}>
                      {swapResult.simulation.success ? "Passed" : "Failed"}
                    </span>
                  </div>
                  
                  {swapResult.simulation.success && !txHash && (
                    <button onClick={handleBroadcast} disabled={loading} className="w-full mt-2 py-3 bg-emerald-500 hover:bg-emerald-400 disabled:opacity-50 text-white rounded-lg font-semibold transition">
                      {broadcastStatus?.includes("signature") ? broadcastStatus : "Sign & Broadcast"}
                    </button>
                  )}
                </div>
              )}

              {txHash && (
                <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-4 text-center space-y-2">
                  <ShieldCheck size={24} className="text-emerald-400 mx-auto" />
                  <p className="text-emerald-400 font-semibold text-sm">Transaction Broadcast</p>
                  <a href={`https://sepolia.etherscan.io/tx/${txHash}`} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-xs text-slate-400 hover:text-white transition break-all font-mono">
                    {txHash.slice(0, 10)}...{txHash.slice(-8)} <ExternalLink size={12} />
                  </a>
                </div>
              )}
            </div>
          </div>
        )}

        {status && (
          <div className="mt-6 text-center text-sm font-medium text-slate-400 bg-white/5 py-3 rounded-xl border border-white/5">{status}</div>
        )}
      </main>

      {/* Expanded Compliance Disclosures */}
      <footer className="border-t border-white/5 bg-[#0B0E14] py-8 mt-auto">
        <div className="max-w-lg mx-auto px-4 space-y-4">
          <p className="text-[11px] text-slate-500 leading-relaxed text-center">
            <strong>Non-Custodial Software.</strong> This interface is purely software and does not hold, control, or manage your private keys or funds. All transactions are constructed locally and signed exclusively by your wallet. You are solely responsible for your activity and compliance with local laws.
          </p>
          <p className="text-[11px] text-slate-500 leading-relaxed text-center">
            <strong>No Financial Advice.</strong> Nothing displayed on this platform constitutes financial, legal, or tax advice. Rates, routes, and fees are provided as neutral data feeds from decentralized networks and may change at any time. We do not guarantee the best price or optimal routing.
          </p>
          <p className="text-[11px] text-slate-500 leading-relaxed text-center">
            <strong>Network Fees.</strong> You are solely responsible for paying all blockchain network fees (gas) associated with your transactions.
          </p>
        </div>
      </footer>
    </div>
  );
}
