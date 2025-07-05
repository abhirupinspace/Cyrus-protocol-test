export interface ChainConfig {
    name: string;
    chainId: string;
    rpc: string;
    networkType: 'mainnet' | 'testnet' | 'devnet';
    programId?: string;
    contractAddress?: string;
    gasSettings: {
      maxRetries?: number;
      commitment?: string;
      maxGasAmount?: number;
      gasUnitPrice?: number;
    };
  }
  
  export interface ChainsConfig {
    chains: ChainConfig[];
    relayer: {
      maxRetries: number;
      retryDelaySeconds: number;
      batchSize: number;
      healthCheckInterval: number;
    };
  }
  
  export interface ContractDeployment {
    address: string;
    deployedAt: string;
    network: string;
    version: string;
  }
  
  export interface DeployedContracts {
    solana: {
      CyrusSettlementEmitter: string;
      deployedAt: string;
      network: string;
      programVersion: string;
    };
    aptos: {
      SettlementProcessor: string;
      VaultOwner: string;
      deployedAt: string;
      network: string;
      contractVersion: string;
    };
    relayer: {
      authorizedRelayers: string[];
      configVersion: string;
    };
  }
  
  export interface SettlementInstruction {
    protocolVersion: number;
    sourceChain: string;
    sourceTxHash: string;
    settlementId: string;
    sender: string;
    receiver: string;
    asset: string;
    amount: number;
    nonce: number;
    timestamp: number;
    expiry: number;
    signature?: string;
  }
  
  export interface SettlementResult {
    success: boolean;
    settlementId: string;
    aptosTransactionHash?: string;
    error?: string;
    processedAt: string;
    gasUsed?: number;
  }
  
  export interface RelayerConfig {
    general: {
      logLevel: string;
      metricsEnabled: boolean;
      healthCheckPort: number;
    };
    solana: {
      rpcUrl: string;
      commitment: string;
      programId: string;
      pollIntervalMs: number;
    };
    aptos: {
      rpcUrl: string;
      contractAddress: string;
      vaultOwner: string;
      maxGasAmount: number;
      gasUnitPrice: number;
    };
    processing: {
      maxConcurrentSettlements: number;
      batchSize: number;
      retryAttempts: number;
      retryDelaySeconds: number;
    };
    monitoring: {
      enableMetrics: boolean;
      metricsPort: number;
      dashboardEnabled: boolean;
      alertWebhookUrl: string;
    };
  }
  
  export interface SettlementEvent {
    eventType: 'SettlementRequested' | 'SettlementCompleted' | 'SettlementFailed';
    settlementId: string;
    sourceChain: string;
    destinationChain: string;
    amount: number;
    timestamp: number;
    data: any;
  }
  
  export interface MonitoringMetrics {
    totalSettlements: number;
    successfulSettlements: number;
    failedSettlements: number;
    averageSettlementTime: number;
    lastSettlementTime: number;
    relayerHealth: 'healthy' | 'degraded' | 'down';
    vaultBalance: number;
  }
  
  export interface HealthCheck {
    service: string;
    status: 'healthy' | 'unhealthy';
    lastCheck: string;
    details?: string;
  }
  
  export interface ApiResponse<T = any> {
    success: boolean;
    data?: T;
    error?: string;
    timestamp: string;
  }