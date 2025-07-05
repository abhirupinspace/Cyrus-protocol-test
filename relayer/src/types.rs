use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Core protocol types and definitions for Cyrus cross-chain settlement

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionHash(pub String);

impl fmt::Display for TransactionHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub String);

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Settlement instruction from source chain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub source_chain: ChainId,
    pub source_tx_hash: TransactionHash,
    pub destination_chain: ChainId,
    pub sender: Address,
    pub receiver: Address,
    pub token_symbol: String,
    pub amount: u64, // Amount in smallest unit (e.g., micro USDC)
    pub nonce: u64,
    pub timestamp: DateTime<Utc>,
    pub payload: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

impl SettlementInstruction {
    pub fn new(
        source_chain: ChainId,
        source_tx_hash: TransactionHash,
        destination_chain: ChainId,
        sender: Address,
        receiver: Address,
        token_symbol: String,
        amount: u64,
        nonce: u64,
        timestamp: DateTime<Utc>,
        payload: Option<Vec<u8>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_chain,
            source_tx_hash,
            destination_chain,
            sender,
            receiver,
            token_symbol,
            amount,
            nonce,
            timestamp,
            payload,
            created_at: Utc::now(),
        }
    }

    pub fn amount_in_usdc(&self) -> f64 {
        self.amount as f64 / 1_000_000.0
    }

    pub fn validate(&self) -> Result<(), SettlementError> {
        if self.source_tx_hash.0.is_empty() {
            return Err(SettlementError::InvalidInstruction("Empty source transaction hash".to_string()));
        }
        
        if self.receiver.0.is_empty() {
            return Err(SettlementError::InvalidInstruction("Empty receiver address".to_string()));
        }
        
        if self.amount == 0 {
            return Err(SettlementError::InvalidInstruction("Amount must be greater than 0".to_string()));
        }
        
        if !self.receiver.0.starts_with("0x") {
            return Err(SettlementError::InvalidInstruction("Invalid receiver address format".to_string()));
        }
        
        Ok(())
    }
}

/// Settlement processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    pub instruction_id: Uuid,
    pub status: SettlementStatus,
    pub destination_tx_hash: Option<TransactionHash>,
    pub gas_used: Option<u64>,
    pub error_message: Option<String>,
    pub processed_at: DateTime<Utc>,
    pub retry_count: u32,
}

impl SettlementResult {
    pub fn success(instruction_id: Uuid, tx_hash: TransactionHash, gas_used: Option<u64>) -> Self {
        Self {
            instruction_id,
            status: SettlementStatus::Completed,
            destination_tx_hash: Some(tx_hash),
            gas_used,
            error_message: None,
            processed_at: Utc::now(),
            retry_count: 0,
        }
    }

    pub fn failure(instruction_id: Uuid, error: String, retry_count: u32) -> Self {
        Self {
            instruction_id,
            status: SettlementStatus::Failed,
            destination_tx_hash: None,
            gas_used: None,
            error_message: Some(error),
            processed_at: Utc::now(),
            retry_count,
        }
    }

    pub fn pending(instruction_id: Uuid) -> Self {
        Self {
            instruction_id,
            status: SettlementStatus::Pending,
            destination_tx_hash: None,
            gas_used: None,
            error_message: None,
            processed_at: Utc::now(),
            retry_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Retrying,
}

/// Solana event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaSettlementEvent {
    pub source_chain: String,
    pub aptos_recipient: String,
    pub amount: u64,
    pub nonce: u64,
    pub slot: u64,
    pub timestamp: u64,
    pub signature: String,
    pub block_time: Option<i64>,
}

impl From<SolanaSettlementEvent> for SettlementInstruction {
    fn from(event: SolanaSettlementEvent) -> Self {
        let timestamp = if let Some(block_time) = event.block_time {
            DateTime::from_timestamp(block_time, 0).unwrap_or_else(Utc::now)
        } else {
            DateTime::from_timestamp(event.timestamp as i64, 0).unwrap_or_else(Utc::now)
        };

        SettlementInstruction::new(
            ChainId("solana".to_string()),
            TransactionHash(event.signature),
            ChainId("aptos".to_string()),
            Address("solana_program".to_string()), // Placeholder for sender
            Address(event.aptos_recipient),
            "USDC".to_string(),
            event.amount,
            event.nonce,
            timestamp,
            None,
        )
    }
}

/// Relayer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub solana: SolanaConfig,
    pub aptos: AptosConfig,
    pub processing: ProcessingConfig,
    pub monitoring: MonitoringConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub program_id: String,
    pub commitment: String,
    pub poll_interval_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptosConfig {
    pub rpc_url: String,
    pub contract_address: String,
    pub vault_owner: String,
    pub private_key: String,
    pub max_gas_amount: u64,
    pub gas_unit_price: u64,
    pub transaction_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub max_concurrent_settlements: usize,
    pub batch_size: usize,
    pub retry_attempts: u32,
    pub retry_delay_seconds: u64,
    pub settlement_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub health_check_port: u16,
    pub log_level: String,
    pub enable_metrics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum SettlementError {
    #[error("Invalid settlement instruction: {0}")]
    InvalidInstruction(String),
    
    #[error("Already processed: {0}")]
    AlreadyProcessed(String),
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Chain error: {0}")]
    ChainError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<serde_json::Error> for SettlementError {
    fn from(err: serde_json::Error) -> Self {
        SettlementError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for SettlementError {
    fn from(err: reqwest::Error) -> Self {
        SettlementError::NetworkError(err.to_string())
    }
}

impl From<sqlx::Error> for SettlementError {
    fn from(err: sqlx::Error) -> Self {
        SettlementError::DatabaseError(err.to_string())
    }
}

/// Metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelayerMetrics {
    pub total_settlements_processed: u64,
    pub successful_settlements: u64,
    pub failed_settlements: u64,
    pub pending_settlements: u64,
    pub average_processing_time_ms: f64,
    pub last_processed_at: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
    pub vault_balance_usdc: f64,
    pub total_volume_usdc: f64,
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub status: ServiceStatus,
    pub last_check: DateTime<Utc>,
    pub details: Option<String>,
    pub response_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// API Response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub request_id: Uuid,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
            request_id: Uuid::new_v4(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
            request_id: Uuid::new_v4(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_instruction_validation() {
        let valid_instruction = SettlementInstruction::new(
            ChainId("solana".to_string()),
            TransactionHash("test_tx".to_string()),
            ChainId("aptos".to_string()),
            Address("sender".to_string()),
            Address("0x123".to_string()),
            "USDC".to_string(),
            1000000,
            1,
            Utc::now(),
            None,
        );

        assert!(valid_instruction.validate().is_ok());

        let invalid_instruction = SettlementInstruction::new(
            ChainId("solana".to_string()),
            TransactionHash("".to_string()),
            ChainId("aptos".to_string()),
            Address("sender".to_string()),
            Address("invalid".to_string()),
            "USDC".to_string(),
            0,
            1,
            Utc::now(),
            None,
        );

        assert!(invalid_instruction.validate().is_err());
    }

    #[test]
    fn test_settlement_result_creation() {
        let instruction_id = Uuid::new_v4();
        let tx_hash = TransactionHash("0x123".to_string());
        
        let success_result = SettlementResult::success(instruction_id, tx_hash, Some(1000));
        assert_eq!(success_result.status, SettlementStatus::Completed);
        assert!(success_result.destination_tx_hash.is_some());

        let failure_result = SettlementResult::failure(instruction_id, "Error".to_string(), 1);
        assert_eq!(failure_result.status, SettlementStatus::Failed);
        assert!(failure_result.error_message.is_some());
    }

    #[test]
    fn test_solana_event_conversion() {
        let event = SolanaSettlementEvent {
            source_chain: "solana".to_string(),
            aptos_recipient: "0x123".to_string(),
            amount: 1000000,
            nonce: 42,
            slot: 12345,
            timestamp: 1640995200,
            signature: "test_signature".to_string(),
            block_time: Some(1640995200),
        };

        let instruction: SettlementInstruction = event.into();
        assert_eq!(instruction.amount, 1000000);
        assert_eq!(instruction.nonce, 42);
        assert_eq!(instruction.receiver.0, "0x123");
    }
}