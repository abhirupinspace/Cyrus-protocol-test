"use client";

import React, {
  useState,
  useEffect,
  useRef,
  useCallback,
  useMemo,
} from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { X, Settings, Receipt, FileText, Wallet } from "lucide-react";
import {
  ConnectionProvider,
  WalletProvider,
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react";
import { WalletAdapterNetwork } from "@solana/wallet-adapter-base";
import {
  WalletModalProvider,
  WalletMultiButton,
} from "@solana/wallet-adapter-react-ui";
import {
  clusterApiUrl,
  PublicKey,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  TorusWalletAdapter,
  LedgerWalletAdapter,
} from "@solana/wallet-adapter-wallets";
require("@solana/wallet-adapter-react-ui/styles.css");

const customScrollbarStyles = `
  .custom-scrollbar::-webkit-scrollbar {
    width: 8px;
  }
  
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 9999px;
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
  }
  
  .custom-scrollbar::-webkit-scrollbar-button {
    display: none;
  }
  
  .custom-scrollbar {
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
  }

  @keyframes fade-in-up {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-fade-in-up {
    animation: fade-in-up 0.5s ease-out forwards;
  }

  .animation-delay-200 {
    animation-delay: 200ms;
  }
`;

interface SettlementLog {
  id: string;
  timestamp: string;
  type: "info" | "success" | "warning" | "error";
  message: string;
  chain?: "solana" | "aptos" | "relayer";
}

interface SettlementStatus {
  step: number;
  status: "pending" | "processing" | "completed" | "failed";
  description: string;
}

const STEPS: SettlementStatus[] = [
  {
    step: 1,
    status: "pending",
    description: "Initialize Settlement on Solana",
  },
  { step: 2, status: "pending", description: "Relayer Detects Event" },
  { step: 3, status: "pending", description: "Sign Settlement Intent" },
  { step: 4, status: "pending", description: "Submit to Aptos Contract" },
  { step: 5, status: "pending", description: "Verify Settlement Complete" },
];

function DashboardInner() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();
  const [settlementView, setSettlementView] = useState<"form" | "transactions">(
    "form"
  );
  const [logs, setLogs] = useState<SettlementLog[]>([]);
  const [steps, setSteps] = useState<SettlementStatus[]>(STEPS);
  const [isProcessing, setIsProcessing] = useState(false);
  const [settlementData, setSettlementData] = useState({
    amount: "",
    aptosReceiver: "",
    solanaTxHash: "",
    aptosTxHash: "",
  });
  const logsEndRef = useRef<HTMLDivElement>(null);
  const progressEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);
  useEffect(() => {
    progressEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [steps]);

  const addLog = useCallback(
    (
      type: SettlementLog["type"],
      message: string,
      chain?: SettlementLog["chain"]
    ) => {
      const newLog: SettlementLog = {
        id: Math.random().toString(36).substring(2, 11),
        timestamp: new Date().toLocaleTimeString(),
        type,
        message,
        chain,
      };
      setLogs((prev) => [...prev, newLog]);
    },
    []
  );

  const updateStep = useCallback(
    (stepIndex: number, status: SettlementStatus["status"]) => {
      setSteps((prev) =>
        prev.map((step, index) =>
          index === stepIndex ? { ...step, status } : step
        )
      );
    },
    []
  );

  const simulateRelayerProcess = useCallback(
    async (solanaTx: string) => {
      updateStep(1, "processing");
      addLog(
        "info",
        "Scanning Solana logs for settlement events...",
        "relayer"
      );
      await new Promise((resolve) => setTimeout(resolve, 2000));
      addLog(
        "success",
        `Found settlement event in transaction: ${solanaTx.slice(0, 8)}...`,
        "relayer"
      );
      updateStep(1, "completed");
      updateStep(2, "processing");
      addLog("info", "Creating settlement intent...", "relayer");
      const settlementIntent = {
        from_chain: "solana",
        to_chain: "aptos",
        source_tx: solanaTx,
        receiver: settlementData.aptosReceiver,
        amount: parseFloat(settlementData.amount) * 1_000_000,
        nonce: Date.now(),
        timestamp: Math.floor(Date.now() / 1000),
      };
      await new Promise((resolve) => setTimeout(resolve, 1500));
      addLog(
        "info",
        `Intent created: ${JSON.stringify(settlementIntent, null, 2)}`,
        "relayer"
      );
      addLog("info", "Signing intent with Ed25519 key...", "relayer");
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const mockSignature =
        "ed25519_sig_" + Math.random().toString(36).substring(2, 18);
      addLog("success", `Intent signed: ${mockSignature}`, "relayer");
      updateStep(2, "completed");
      updateStep(3, "processing");
      addLog("info", "Submitting settlement to Aptos contract...", "aptos");
      await new Promise((resolve) => setTimeout(resolve, 2500));
      const mockAptosTx = "0x" + Math.random().toString(16).substring(2, 66);
      setSettlementData((prev) => ({ ...prev, aptosTxHash: mockAptosTx }));
      addLog("success", `Aptos transaction submitted: ${mockAptosTx}`, "aptos");
      addLog("info", "Waiting for transaction confirmation...", "aptos");
      await new Promise((resolve) => setTimeout(resolve, 2000));
      addLog("success", "Settlement executed successfully on Aptos!", "aptos");
      updateStep(3, "completed");
      updateStep(4, "processing");
      addLog("info", "Verifying settlement completion...", "relayer");
      await new Promise((resolve) => setTimeout(resolve, 1500));
      addLog("success", "Settlement verified and completed!", "relayer");
      addLog(
        "info",
        `USDC transferred to ${settlementData.aptosReceiver.slice(0, 10)}...`,
        "aptos"
      );
      updateStep(4, "completed");
      setIsProcessing(false);
      setSettlementView("transactions");
    },
    [addLog, updateStep, settlementData.aptosReceiver, settlementData.amount]
  );

  const handleInitiateSettlement = useCallback(async () => {
    if (!connected || !publicKey) {
      addLog("error", "Please connect your wallet first");
      return;
    }
    if (!settlementData.amount || !settlementData.aptosReceiver) {
      addLog("error", "Please enter amount and Aptos receiver address");
      return;
    }
    try {
      setIsProcessing(true);
      setLogs([]);
      setSteps(STEPS.map((step) => ({ ...step, status: "pending" })));
      updateStep(0, "processing");
      addLog("info", "Creating Solana settlement transaction...", "solana");
      const transaction = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: publicKey,
          toPubkey: new PublicKey("11111111111111111111111111111112"),
          lamports: 1000,
        })
      );
      addLog("info", "Sending transaction to Solana...", "solana");
      const signature = await sendTransaction(transaction, connection);
      setSettlementData((prev) => ({ ...prev, solanaTxHash: signature }));
      addLog("success", `Solana transaction sent: ${signature}`, "solana");
      addLog("info", "Waiting for Solana confirmation...", "solana");
      await connection.confirmTransaction(signature);
      addLog("success", "Solana transaction confirmed!", "solana");
      addLog(
        "info",
        `Settlement event emitted for ${settlementData.amount} USDC`,
        "solana"
      );
      updateStep(0, "completed");
      await simulateRelayerProcess(signature);
    } catch (error) {
      addLog(
        "error",
        `Transaction failed: ${
          error instanceof Error ? error.message : "Unknown error"
        }`
      );
      setIsProcessing(false);
      setSteps((prev) =>
        prev.map((step) =>
          step.status === "processing" ? { ...step, status: "failed" } : step
        )
      );
    }
  }, [
    connected,
    publicKey,
    sendTransaction,
    connection,
    addLog,
    updateStep,
    settlementData.amount,
    settlementData.aptosReceiver,
    simulateRelayerProcess,
  ]);

  const resetDemo = useCallback(() => {
    setLogs([]);
    setSteps(STEPS.map((step) => ({ ...step, status: "pending" })));
    setSettlementData((prev) => ({
      ...prev,
      solanaTxHash: "",
      aptosTxHash: "",
    }));
    setIsProcessing(false);
    setSettlementView("form");
  }, []);

  // UI helpers
  const getLogColor = (type: SettlementLog["type"]) => {
    switch (type) {
      case "success":
        return "text-green-400";
      case "warning":
        return "text-yellow-400";
      case "error":
        return "text-red-400";
      default:
        return "text-blue-400";
    }
  };
  const getStepColor = (status: SettlementStatus["status"]) => {
    switch (status) {
      case "completed":
        return "text-green-400";
      case "processing":
        return "text-blue-400";
      case "failed":
        return "text-red-400";
      default:
        return "text-slate-400";
    }
  };
  const getStepDot = (status: SettlementStatus["status"]) => {
    switch (status) {
      case "completed":
        return "bg-green-400";
      case "processing":
        return "bg-blue-400 animate-pulse";
      case "failed":
        return "bg-red-400";
      default:
        return "bg-slate-400";
    }
  };

  // Transaction links
  const transactionLinks = [
    settlementData.solanaTxHash && {
      id: "sol",
      hash: settlementData.solanaTxHash,
      type: "Solana",
    },
    settlementData.aptosTxHash && {
      id: "aptos",
      hash: settlementData.aptosTxHash,
      type: "Aptos",
    },
  ].filter(Boolean) as { id: string; hash: string; type: string }[];

  return (
    <div
      className="min-h-screen bg-cover bg-center bg-no-repeat relative"
      style={{ backgroundImage: "url(/images/bg.png)" }}
    >
      <div className="absolute inset-0 bg-black/70"></div>
      <nav className="fixed top-4 left-1/2 transform -translate-x-1/2 w-full max-w-6xl h-[9vh] z-50">
        <div className="bg-white/5 border border-white/20 backdrop-blur-2xl rounded-2xl shadow-2xl h-full flex items-center justify-between px-6">
          <div className="flex items-center">
            <div className="flex items-center gap-3">
              <img
                src="/images/icon.png"
                alt="Cyrus Protocol Logo"
                className="w-8 h-8"
              />
              <div className="flex flex-col">
                <h1 className="text-white text-2xl font-bold tracking-wide">
                  Cyrus Protocol
                </h1>
                <p className="text-slate-300 text-sm font-medium">
                  Cross Chain Settlement Demo
                </p>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <WalletMultiButton />
          </div>
        </div>
      </nav>
      <style dangerouslySetInnerHTML={{ __html: customScrollbarStyles }} />
      <div className="flex items-center justify-center p-4 pt-[calc(9vh+2rem)]">
        <div className="w-full max-w-6xl grid grid-cols-2 gap-6 h-[80vh] relative z-10">
          {/* Left Column - Transaction Logs */}
          <div className="col-span-1 row-span-1 h-[80vh]">
            <div className="bg-white/2 border border-white/5 backdrop-blur-2xl rounded-xl p-4 h-[80vh] shadow-2xl">
              <Card className="bg-white/5 border border-white/5 backdrop-blur-3xl shadow-2xl h-full flex flex-col">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
                  <CardTitle className="text-white text-lg font-medium">
                    Transaction Logs
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant="secondary"
                      className="bg-green-500/20 text-green-400 text-xs"
                    >
                      Live
                    </Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 text-slate-400"
                    >
                      <Settings className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 text-slate-400"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                </CardHeader>
                <CardContent className="space-y-2 flex-1 overflow-y-auto max-h-full custom-scrollbar">
                  <div className="space-y-2 text-sm font-mono">
                    {logs.length === 0 ? (
                      <div className="text-slate-400">
                        Waiting for settlement to begin...
                      </div>
                    ) : (
                      logs.map((log, index) => (
                        <div
                          key={log.id}
                          className="flex items-start gap-3 p-3 bg-white/5 rounded-lg border border-white/15 animate-fade-in-up backdrop-blur-sm"
                          style={{ animationDelay: `${index * 100}ms` }}
                        >
                          <div
                            className={`w-2 h-2 rounded-full mt-2 flex-shrink-0 ${getLogColor(
                              log.type
                            ).replace("text-", "bg-")}`}
                          ></div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center justify-between mb-1">
                              <span
                                className={`font-medium ${getLogColor(
                                  log.type
                                )}`}
                              >
                                {log.type.toUpperCase()}
                              </span>
                              <span className="text-slate-400 text-xs">
                                {log.timestamp}
                              </span>
                            </div>
                            <p className="text-white break-words whitespace-pre-wrap max-w-full overflow-x-auto">
                              {log.message}
                            </p>
                          </div>
                        </div>
                      ))
                    )}
                    <div ref={logsEndRef} />
                  </div>
                </CardContent>
              </Card>
            </div>
          </div>
          {/* Right Column - Config & Progress */}
          <div className="col-span-1 grid grid-rows-2 gap-6 h-[80vh]">
            {/* Top Row - Settlement Config */}
            <div className="bg-white/2 border border-white/15 backdrop-blur-2xl rounded-xl p-4 h-full overflow-hidden shadow-2xl">
              <Card className="bg-white/5 border border-white/20 backdrop-blur-3xl shadow-2xl h-full flex flex-col overflow-hidden">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
                  <CardTitle className="text-white text-lg font-medium">
                    Settlement Configuration
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    {transactionLinks.length > 0 && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0 text-slate-400 hover:text-white transition-colors"
                        onClick={() =>
                          setSettlementView(
                            settlementView === "form" ? "transactions" : "form"
                          )
                        }
                      >
                        {settlementView === "form" ? (
                          <Receipt className="h-4 w-4" />
                        ) : (
                          <FileText className="h-4 w-4" />
                        )}
                      </Button>
                    )}
                  </div>
                </CardHeader>
                <CardContent className="flex-1 px-4">
                  <div
                    className={`transition-all duration-300 ease-in-out ${
                      settlementView === "form"
                        ? "opacity-100 translate-y-0"
                        : "opacity-0 -translate-y-2 absolute"
                    }`}
                  >
                    {settlementView === "form" && (
                      <div className="space-y-3">
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                          <div className="space-y-2">
                            <label className="text-slate-300 text-sm font-medium">
                              USDC Amount
                            </label>
                            <input
                              type="number"
                              placeholder="100"
                              value={settlementData.amount}
                              onChange={(e) =>
                                setSettlementData((prev) => ({
                                  ...prev,
                                  amount: e.target.value,
                                }))
                              }
                              className="w-full bg-white/5 border border-white/20 backdrop-blur-sm text-white rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-purple-400 focus:border-purple-400 transition-all duration-200 placeholder:text-slate-400"
                              disabled={isProcessing}
                            />
                          </div>
                          <div className="space-y-2 md:col-span-2">
                            <label className="text-slate-300 text-sm font-medium">
                              Aptos Receiver Account
                            </label>
                            <input
                              type="text"
                              placeholder="Enter Aptos account address"
                              value={settlementData.aptosReceiver}
                              onChange={(e) =>
                                setSettlementData((prev) => ({
                                  ...prev,
                                  aptosReceiver: e.target.value,
                                }))
                              }
                              className="w-full bg-white/5 border border-white/20 backdrop-blur-sm text-white rounded-xl px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-purple-400 focus:border-purple-400 transition-all duration-200 placeholder:text-slate-400"
                              disabled={isProcessing}
                            />
                          </div>
                        </div>
                        <button
                          onClick={handleInitiateSettlement}
                          disabled={
                            !connected ||
                            !publicKey ||
                            !settlementData.amount ||
                            !settlementData.aptosReceiver ||
                            isProcessing
                          }
                          className="w-full bg-gradient-to-r from-purple-500 to-purple-600 hover:from-purple-600 hover:to-purple-700 disabled:hover:from-purple-500 disabled:hover:to-purple-600 disabled:cursor-not-allowed text-white font-medium py-3 px-6 rounded-full transition-all duration-200 transform hover:scale-[1.02] disabled:hover:scale-100 shadow-lg hover:shadow-purple-500/25 disabled:hover:shadow-lg"
                        >
                          {isProcessing ? "Processing..." : "Send Transaction"}
                        </button>
                        <button
                          onClick={resetDemo}
                          disabled={isProcessing}
                          className="w-full mt-2 bg-white/5 border border-white/20 text-white rounded-full py-2 px-4 hover:bg-white/20 disabled:opacity-50 transition-colors"
                        >
                          Reset
                        </button>
                      </div>
                    )}
                  </div>
                  <div
                    className={`transition-all duration-300 ease-in-out ${
                      settlementView === "transactions"
                        ? "opacity-100 translate-y-0"
                        : "opacity-0 translate-y-2 absolute"
                    }`}
                  >
                    {settlementView === "transactions" &&
                      steps.every(
                        (s) =>
                          s.status === "completed" || s.status === "pending"
                      ) &&
                      steps[steps.length - 1].status === "completed" && (
                        <div className="space-y-2">
                          {transactionLinks.map((link, index) => (
                            <div
                              key={link.id}
                              className={`flex items-center gap-3 bg-white/15 border border-white/25 backdrop-blur-md rounded-lg px-4 py-3 transition-all duration-300 hover:bg-white/20 hover:border-white/35 transform hover:scale-[1.02] ${
                                index === 0
                                  ? "animate-fade-in-up"
                                  : "animate-fade-in-up animation-delay-200"
                              }`}
                              style={{ animationDelay: `${index * 200}ms` }}
                            >
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center justify-between">
                                  <span className="text-white text-sm font-medium">
                                    {link.type} Transaction
                                  </span>
                                  <a
                                    href={
                                      link.type === "Solana"
                                        ? `https://explorer.solana.com/tx/${link.hash}?cluster=devnet`
                                        : `https://explorer.aptoslabs.com/txn/${link.hash}?network=testnet`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-slate-300 hover:text-white transition-colors"
                                  >
                                    <svg
                                      className="w-4 h-4"
                                      viewBox="0 0 24 24"
                                      fill="none"
                                      stroke="currentColor"
                                      strokeWidth="2"
                                    >
                                      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                                      <polyline points="15,3 21,3 21,9" />
                                      <line x1="10" y1="14" x2="21" y2="3" />
                                    </svg>
                                  </a>
                                </div>
                                <span className="text-slate-300 text-xs font-mono break-all whitespace-pre-wrap max-w-full overflow-x-auto">
                                  {link.hash}
                                </span>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                  </div>
                </CardContent>
              </Card>
            </div>
            {/* Bottom Row - Settlement Progress */}
            <div className="bg-white/2 border border-white/15 backdrop-blur-2xl rounded-xl p-4 h-full shadow-2xl">
              <Card className="bg-white/5 border border-white/20 backdrop-blur-3xl shadow-2xl h-full flex flex-col">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
                  <CardTitle className="text-white text-lg font-medium">
                    Settlement Progress
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant="secondary"
                      className={`text-xs ${
                        steps.some((s) => s.status !== "pending")
                          ? "bg-blue-500/20 text-blue-400"
                          : "bg-slate-500/20 text-slate-400"
                      }`}
                    >
                      {steps.some((s) => s.status !== "pending")
                        ? "Active"
                        : "Waiting"}
                    </Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 text-slate-400"
                    >
                      <Settings className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 text-slate-400"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                </CardHeader>
                <CardContent className="space-y-2 flex-1 overflow-y-auto max-h-full custom-scrollbar">
                  {!steps.some((item) => item.status === "completed") ? (
                    <div className="flex items-center justify-center h-full">
                      <p className="text-slate-400 text-sm">
                        Settlement progress will appear here after sending
                        transaction
                      </p>
                    </div>
                  ) : (
                    <div className="space-y-2 text-sm">
                      {steps.map((progress, index) =>
                        progress.status !== "pending" ? (
                          <div
                            key={progress.step}
                            className="flex items-start gap-3 p-3 bg-white/5 rounded-lg border border-white/15 animate-fade-in-up backdrop-blur-sm"
                            style={{ animationDelay: `${index * 200}ms` }}
                          >
                            <div
                              className={`w-2 h-2 rounded-full mt-2 flex-shrink-0 ${getStepDot(
                                progress.status
                              )}`}
                            ></div>
                            <div className="flex-1">
                              <div className="flex items-center justify-between mb-1">
                                <span
                                  className={`font-medium ${getStepColor(
                                    progress.status
                                  )}`}
                                >
                                  {progress.status === "completed"
                                    ? "COMPLETED"
                                    : progress.status === "processing"
                                    ? "IN PROGRESS"
                                    : progress.status === "failed"
                                    ? "ERROR"
                                    : "PENDING"}
                                </span>
                              </div>
                              <p className="text-white">
                                Step {progress.step}: {progress.description}
                              </p>
                              {progress.status === "processing" && (
                                <p className="text-slate-400 text-xs mt-1">
                                  Processing...
                                </p>
                              )}
                              {progress.status === "completed" &&
                                progress.step === 5 && (
                                  <p className="text-slate-400 text-xs mt-1">
                                    Settlement successfully completed
                                  </p>
                                )}
                            </div>
                          </div>
                        ) : (
                          <></>
                        )
                      )}
                      <div ref={progressEndRef} />
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function Dashboard() {
  const network = WalletAdapterNetwork.Devnet;
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter({ network }),
      new TorusWalletAdapter(),
      new LedgerWalletAdapter(),
    ],
    [network]
  );
  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <DashboardInner />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
