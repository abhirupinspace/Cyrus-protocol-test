'use client';
import React, { FC, useState, useCallback, useMemo } from 'react';
import { ConnectionProvider, WalletProvider, useConnection, useWallet } from '@solana/wallet-adapter-react';
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { 
  WalletModalProvider, 
  WalletDisconnectButton, 
  WalletMultiButton 
} from '@solana/wallet-adapter-react-ui';
import { clusterApiUrl } from '@solana/web3.js';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  TorusWalletAdapter,
  LedgerWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { PublicKey, Transaction, SystemProgram } from '@solana/web3.js';

// Default styles that can be overridden by your app
require('@solana/wallet-adapter-react-ui/styles.css');

interface SettlementLog {
  id: string;
  timestamp: string;
  type: 'info' | 'success' | 'warning' | 'error';
  message: string;
  chain?: 'solana' | 'aptos' | 'relayer';
}

interface SettlementStatus {
  step: number;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  description: string;
}

const STEPS: SettlementStatus[] = [
  { step: 1, status: 'pending', description: 'Initialize Settlement on Solana' },
  { step: 2, status: 'pending', description: 'Relayer Detects Event' },
  { step: 3, status: 'pending', description: 'Sign Settlement Intent' },
  { step: 4, status: 'pending', description: 'Submit to Aptos Contract' },
  { step: 5, status: 'pending', description: 'Verify Settlement Complete' },
];

const SettlementDemo: FC = () => {
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();
  const [logs, setLogs] = useState<SettlementLog[]>([]);
  const [steps, setSteps] = useState<SettlementStatus[]>(STEPS);
  const [isProcessing, setIsProcessing] = useState(false);
  const [settlementData, setSettlementData] = useState({
    amount: '1.0',
    aptosReceiver: '0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd',
    solanaTxHash: '',
    aptosTxHash: '',
  });

  const addLog = useCallback((type: SettlementLog['type'], message: string, chain?: SettlementLog['chain']) => {
    const newLog: SettlementLog = {
      id: Math.random().toString(36).substring(2, 11),
      timestamp: new Date().toLocaleTimeString(),
      type,
      message,
      chain,
    };
    setLogs(prev => [...prev, newLog]);
  }, []);

  const updateStep = useCallback((stepIndex: number, status: SettlementStatus['status']) => {
    setSteps(prev => prev.map((step, index) => 
      index === stepIndex ? { ...step, status } : step
    ));
  }, []);

  const simulateRelayerProcess = useCallback(async (solanaTx: string) => {
    // Step 2: Relayer detects event
    updateStep(1, 'processing');
    addLog('info', 'Scanning Solana logs for settlement events...', 'relayer');
    
    await new Promise(resolve => setTimeout(resolve, 2000));
    addLog('success', `Found settlement event in transaction: ${solanaTx.slice(0, 8)}...`, 'relayer');
    updateStep(1, 'completed');

    // Step 3: Sign settlement intent
    updateStep(2, 'processing');
    addLog('info', 'Creating settlement intent...', 'relayer');
    
    const settlementIntent = {
      from_chain: 'solana',
      to_chain: 'aptos',
      source_tx: solanaTx,
      receiver: settlementData.aptosReceiver,
      amount: parseFloat(settlementData.amount) * 1_000_000, // Convert to micro USDC
      nonce: Date.now(),
      timestamp: Math.floor(Date.now() / 1000),
    };

    await new Promise(resolve => setTimeout(resolve, 1500));
    addLog('info', `Intent created: ${JSON.stringify(settlementIntent, null, 2)}`, 'relayer');
    addLog('info', 'Signing intent with Ed25519 key...', 'relayer');
    
    await new Promise(resolve => setTimeout(resolve, 1000));
    const mockSignature = 'ed25519_sig_' + Math.random().toString(36).substring(2, 18);
    addLog('success', `Intent signed: ${mockSignature}`, 'relayer');
    updateStep(2, 'completed');

    // Step 4: Submit to Aptos
    updateStep(3, 'processing');
    addLog('info', 'Submitting settlement to Aptos contract...', 'aptos');
    
    await new Promise(resolve => setTimeout(resolve, 2500));
    const mockAptosTx = '0x' + Math.random().toString(16).substring(2, 66);
    setSettlementData(prev => ({ ...prev, aptosTxHash: mockAptosTx }));
    
    addLog('success', `Aptos transaction submitted: ${mockAptosTx}`, 'aptos');
    addLog('info', 'Waiting for transaction confirmation...', 'aptos');
    
    await new Promise(resolve => setTimeout(resolve, 2000));
    addLog('success', 'Settlement executed successfully on Aptos!', 'aptos');
    updateStep(3, 'completed');

    // Step 5: Verify completion
    updateStep(4, 'processing');
    addLog('info', 'Verifying settlement completion...', 'relayer');
    
    await new Promise(resolve => setTimeout(resolve, 1500));
    addLog('success', 'Settlement verified and completed!', 'relayer');
    addLog('info', `USDC transferred to ${settlementData.aptosReceiver.slice(0, 10)}...`, 'aptos');
    updateStep(4, 'completed');
    
    setIsProcessing(false);
  }, [addLog, updateStep, settlementData.aptosReceiver, settlementData.amount]);

  const handleInitiateSettlement = useCallback(async () => {
    if (!connected || !publicKey) {
      addLog('error', 'Please connect your wallet first');
      return;
    }

    try {
      setIsProcessing(true);
      setLogs([]);
      setSteps(STEPS.map(step => ({ ...step, status: 'pending' })));

      // Step 1: Solana transaction
      updateStep(0, 'processing');
      addLog('info', 'Creating Solana settlement transaction...', 'solana');

      // Create a simple transaction (in production, this would call the settlement program)
      const transaction = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: publicKey,
          toPubkey: new PublicKey('11111111111111111111111111111112'), // System program
          lamports: 1000, // Minimal amount for demo
        })
      );

      addLog('info', 'Sending transaction to Solana...', 'solana');
      const signature = await sendTransaction(transaction, connection);
      
      setSettlementData(prev => ({ ...prev, solanaTxHash: signature }));
      addLog('success', `Solana transaction sent: ${signature}`, 'solana');
      
      // Wait for confirmation
      addLog('info', 'Waiting for Solana confirmation...', 'solana');
      await connection.confirmTransaction(signature);
      
      addLog('success', 'Solana transaction confirmed!', 'solana');
      addLog('info', `Settlement event emitted for ${settlementData.amount} USDC`, 'solana');
      updateStep(0, 'completed');

      // Start relayer process
      await simulateRelayerProcess(signature);

    } catch (error) {
      addLog('error', `Transaction failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
      setIsProcessing(false);
      setSteps(prev => prev.map(step => 
        step.status === 'processing' ? { ...step, status: 'failed' } : step
      ));
    }
  }, [connected, publicKey, sendTransaction, connection, addLog, updateStep, settlementData.amount, simulateRelayerProcess]);

  const resetDemo = useCallback(() => {
    setLogs([]);
    setSteps(STEPS.map(step => ({ ...step, status: 'pending' })));
    setSettlementData(prev => ({ ...prev, solanaTxHash: '', aptosTxHash: '' }));
    setIsProcessing(false);
  }, []);

  const getStepIcon = (status: SettlementStatus['status']) => {
    switch (status) {
      case 'completed': return '✅';
      case 'processing': return '⏳';
      case 'failed': return '❌';
      default: return '⭕';
    }
  };

  const getLogIcon = (type: SettlementLog['type']) => {
    switch (type) {
      case 'success': return '✅';
      case 'warning': return '⚠️';
      case 'error': return '❌';
      default: return 'ℹ️';
    }
  };

  const getChainBadge = (chain?: SettlementLog['chain']) => {
    if (!chain) return null;
    
    const styles = {
      solana: 'bg-purple-100 text-purple-800',
      aptos: 'bg-blue-100 text-blue-800',
      relayer: 'bg-green-100 text-green-800',
    };

    return (
      <span className={`px-2 py-1 text-xs rounded-full ${styles[chain]}`}>
        {chain.toUpperCase()}
      </span>
    );
  };
//[#EEBA2B]
  return (
    <div className="min-h-screen bg-yellow-50 font-sans">
      {/* Header */}
      <div className="bg-yellow-50 shadow-lg shadow-amber-100">
        <div className="max-w-6xl mx-auto px-4 py-4">
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">Cyrus Protocol</h1>
              <p className="text-gray-900">Cross-Chain Settlement Demo</p>
            </div>
            <div className="flex gap-2">
              <WalletMultiButton />
              {connected}
            </div>
          </div>
        </div>
      </div>

      

      <div className="max-w-6xl mx-auto px-4 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          
          {/* Settlement Controls */}
          <div className="bg-[#FFF8E0] rounded-lg shadow-sm hover:shadow-md hover:shadow-yellow-200 border p-6">
            <h2 className="text-xl font-semibold mb-4 text-black">Settlement Configuration</h2>
            
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Amount (USDC)
                </label>
                <input
                  type="number"
                  step="0.1"
                  value={settlementData.amount}
                  onChange={(e) => setSettlementData(prev => ({ ...prev, amount: e.target.value }))}
                  className="w-full px-3 py-2 border border-gray-300 text-gray-700
                   rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-300 font-mono text-sm"
                  disabled={isProcessing}
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Aptos Receiver Address
                </label>
                <input
                  type="text"
                  value={settlementData.aptosReceiver}
                  onChange={(e) => setSettlementData(prev => ({ ...prev, aptosReceiver: e.target.value }))}
                  className="w-full px-3 py-2 border border-gray-300 text-gray-700
                   rounded-md focus:outline-none focus:ring-2 focus:ring-yellow-300 font-mono text-sm"
                  disabled={isProcessing}
                />
              </div>

              <div className="flex gap-3">
                <button
                  onClick={handleInitiateSettlement}
                  disabled={!connected || !publicKey || isProcessing}
                  className="flex-1 bg-yellow-400 text-black py-2 px-4 rounded-md hover:bg-yellow-500 hover:text-semi-bold disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
                >
                  {isProcessing ? 'Processing...' : 'Initiate Settlement'}
                </button>
                
                <button
                  onClick={resetDemo}
                  disabled={isProcessing}
                  className="px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 transition-colors text-black"
                >
                  Reset
                </button>
              </div>

              {!connected && (
                <div className="bg-yellow-50 border border-yellow-200 rounded-md p-3">
                  <p className="text-sm text-yellow-800">
                    Please connect your wallet to initiate a settlement.
                  </p>
                </div>
              )}
            </div>

            {/* Transaction Links */}
            {(settlementData.solanaTxHash || settlementData.aptosTxHash) && (
              <div className="mt-6 pt-6 border-t">
                <h3 className="font-medium mb-3">Transaction Links</h3>
                <div className="space-y-2">
                  {settlementData.solanaTxHash && (
                    <div>
                      <a
                        href={`https://explorer.solana.com/tx/${settlementData.solanaTxHash}?cluster=devnet`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 hover:text-blue-800 text-sm break-all"
                      >
                        🔗 Solana: {settlementData.solanaTxHash}
                      </a>
                    </div>
                  )}
                  {settlementData.aptosTxHash && (
                    <div>
                      <a
                        href={`https://explorer.aptoslabs.com/txn/${settlementData.aptosTxHash}?network=testnet`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 hover:text-blue-800 text-sm break-all"
                      >
                        🔗 Aptos: {settlementData.aptosTxHash}
                      </a>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Settlement Progress */}
          <div className="bg-[#FFF8E2] rounded-lg shadow-sm border p-6">
            <h2 className="text-xl text-black font-semibold mb-4">Settlement Progress</h2>
            
            <div className="space-y-3">
              {steps.map((step, index) => (
                <div key={step.step} className="flex items-center gap-3">
                  <span className="text-lg">{getStepIcon(step.status)}</span>
                  <div className="flex-1">
                    <span className={`text-sm ${
                      step.status === 'completed' ? 'text-green-700' :
                      step.status === 'processing' ? 'text-blue-700' :
                      step.status === 'failed' ? 'text-red-700' :
                      'text-gray-600'
                    }`}>
                      Step {step.step}: {step.description}
                    </span>
                  </div>
                  {step.status === 'processing' && (
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Real-time Logs */}
        <div className="mt-8 bg-[#FFF8E2] rounded-lg shadow-sm border">
          
          <div className="p-4 border-b">
            <div className='flex justify-between items-center'>
            <h2 className="text-xl font-semibold text-black">Real-time Settlement Logs</h2>
            {/* Wallet Status */}
            {connected && publicKey && (
              <div className="p-4">
                <div className="max-w-6xl mx-auto">
                  <div className="flex">
                    <div className="flex-shrink-0">
                      <svg className="h-5 w-5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                      </svg>
                    </div>
                    <div className="ml-3">
                      <p className="text-sm text-green-700">
                        Wallet connected: <span className="font-mono">{publicKey.toString().slice(0, 8)}...{publicKey.toString().slice(-8)}</span>
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div></div>
          
          <div className="p-4">
            <div className="bg-gray-900 rounded-lg p-4 h-96 overflow-y-auto font-mono text-sm">
              {logs.length === 0 ? (
                <div className="text-gray-400">Waiting for settlement to begin...</div>
              ) : (
                logs.map((log) => (
                  <div key={log.id} className="mb-2 flex items-start gap-2">
                    <span className="text-gray-400 text-xs">[{log.timestamp}]</span>
                    <span>{getLogIcon(log.type)}</span>
                    {getChainBadge(log.chain)}
                    <span className={`flex-1 ${
                      log.type === 'success' ? 'text-green-400' :
                      log.type === 'warning' ? 'text-yellow-400' :
                      log.type === 'error' ? 'text-red-400' :
                      'text-gray-300'
                    }`}>
                      {log.message}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default function Page() {
  // The network can be set to 'devnet', 'testnet', or 'mainnet-beta'.
  const network = WalletAdapterNetwork.Devnet;

  // You can also provide a custom RPC endpoint.
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
          <SettlementDemo />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}