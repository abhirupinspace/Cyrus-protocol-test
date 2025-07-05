use serde::{Deserialize, Serialize};

/// Settlement instruction from Solana
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub source_tx_hash: String,
    pub receiver: String,
    pub amount: u64,           // Amount in micro USDC (6 decimals)
    pub nonce: u64,
}

impl SettlementInstruction {
    pub fn new(source_tx_hash: String, receiver: String, amount_usdc: f64, nonce: u64) -> Self {
        Self {
            source_tx_hash,
            receiver,
            amount: (amount_usdc * 1_000_000.0) as u64, // Convert to micro USDC
            nonce,
        }
    }
    
    pub fn amount_in_usdc(&self) -> f64 {
        self.amount as f64 / 1_000_000.0
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.source_tx_hash.is_empty() {
            return Err("Source TX hash cannot be empty".to_string());
        }
        if self.receiver.is_empty() || !self.receiver.starts_with("0x") {
            return Err("Invalid receiver address".to_string());
        }
        if self.amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Settlement result
#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub success: bool,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
}

impl SettlementResult {
    pub fn success(tx_hash: String) -> Self {
        Self {
            success: true,
            tx_hash: Some(tx_hash),
            error: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            tx_hash: None,
            error: Some(message),
        }
    }
}

/// Relayer configuration
#[derive(Debug, Clone)]
pub struct RelayerConfig {
    pub contract_address: String,
    pub vault_owner: String,
}

impl RelayerConfig {
    pub fn new(contract_address: String, vault_owner: String) -> Self {
        Self {
            contract_address,
            vault_owner,
        }
    }
}