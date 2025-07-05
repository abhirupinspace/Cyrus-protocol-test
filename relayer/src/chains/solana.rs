use crate::types::{
    SettlementError, SettlementInstruction, SolanaConfig, SolanaSettlementEvent,
    TransactionHash,
};
use async_trait::async_trait;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::{RpcLogsResponse, Response},
};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// Trait for interacting with source chains
#[async_trait]
pub trait SourceChain: Send + Sync {
    async fn start_event_listener(&self) -> Result<(), SettlementError>;
    async fn get_settlement_events(
        &self,
        from_slot: Option<u64>,
    ) -> Result<Vec<SettlementInstruction>, SettlementError>;
    async fn verify_transaction(&self, tx_hash: &TransactionHash) -> Result<bool, SettlementError>;
    async fn get_latest_slot(&self) -> Result<u64, SettlementError>;
}

/// Solana chain implementation
pub struct SolanaChain {
    client: Arc<RpcClient>,
    config: SolanaConfig,
    program_id: Pubkey,
    commitment: CommitmentConfig,
    event_sender: Option<tokio::sync::mpsc::UnboundedSender<SettlementInstruction>>,
}

impl SolanaChain {
    pub fn new(
        config: SolanaConfig,
        event_sender: Option<tokio::sync::mpsc::UnboundedSender<SettlementInstruction>>,
    ) -> Result<Self, SettlementError> {
        let client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            match config.commitment.as_str() {
                "processed" => CommitmentConfig::processed(),
                "confirmed" => CommitmentConfig::confirmed(),
                "finalized" => CommitmentConfig::finalized(),
                _ => CommitmentConfig::confirmed(),
            },
        ));

        let program_id = Pubkey::from_str(&config.program_id)
            .map_err(|e| SettlementError::ConfigError(format!("Invalid program ID: {}", e)))?;

        let commitment = match config.commitment.as_str() {
            "processed" => CommitmentConfig::processed(),
            "confirmed" => CommitmentConfig::confirmed(),
            "finalized" => CommitmentConfig::finalized(),
            _ => CommitmentConfig::confirmed(),
        };

        Ok(Self {
            client,
            config,
            program_id,
            commitment,
            event_sender,
        })
    }

    /// Parse logs to extract settlement events
    fn parse_settlement_event(&self, logs: &[String], signature: &str, slot: u64, block_time: Option<i64>) -> Option<SettlementInstruction> {
        for log in logs {
            if log.contains("SETTLEMENT_EVENT:") {
                if let Some(json_start) = log.find('{') {
                    let json_str = &log[json_start..];
                    match serde_json::from_str::<serde_json::Value>(json_str) {
                        Ok(event_json) => {
                            let event = SolanaSettlementEvent {
                                source_chain: "solana".to_string(),
                                aptos_recipient: event_json["aptos_recipient"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                amount: event_json["amount"].as_u64().unwrap_or(0),
                                nonce: event_json["nonce"].as_u64().unwrap_or(0),
                                slot,
                                timestamp: event_json["timestamp"].as_u64().unwrap_or(0),
                                signature: signature.to_string(),
                                block_time,
                            };

                            debug!("Parsed settlement event: {:?}", event);
                            return Some(event.into());
                        }
                        Err(e) => {
                            warn!("Failed to parse settlement event JSON: {}", e);
                        }
                    }
                }
            }
        }
        None
    }

    /// Process a batch of log responses
    async fn process_log_responses(&self, responses: &[RpcLogsResponse]) -> Vec<SettlementInstruction> {
        let mut instructions = Vec::new();

        for response in responses {
            if let Some(signature) = &response.signature {
                if let Ok(sig) = Signature::from_str(signature) {
                    // Get transaction details
                    match self.client.get_transaction(&sig, solana_client::rpc_config::RpcTransactionConfig {
                        encoding: Some(solana_account_decoder::UiTransactionEncoding::Json),
                        commitment: Some(self.commitment),
                        max_supported_transaction_version: Some(0),
                    }).await {
                        Ok(confirmed_transaction) => {
                            if let Some(transaction) = confirmed_transaction.transaction {
                                if let Some(meta) = transaction.meta {
                                    if let Some(log_messages) = meta.log_messages {
                                        if let Some(instruction) = self.parse_settlement_event(
                                            &log_messages,
                                            signature,
                                            confirmed_transaction.slot,
                                            confirmed_transaction.block_time,
                                        ) {
                                            instructions.push(instruction);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to get transaction details for {}: {}", signature, e);
                        }
                    }
                }
            }
        }

        instructions
    }
}

#[async_trait]
impl SourceChain for SolanaChain {
    async fn start_event_listener(&self) -> Result<(), SettlementError> {
        info!("Starting Solana event listener for program: {}", self.program_id);
        
        let client = Arc::clone(&self.client);
        let program_id = self.program_id;
        let event_sender = self.event_sender.clone();
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);
        let commitment = self.commitment;

        tokio::spawn(async move {
            let mut last_processed_slot = 0u64;
            let mut retry_count = 0u32;
            let max_retries = 3;

            loop {
                match client.get_slot_with_commitment(commitment).await {
                    Ok(current_slot) => {
                        retry_count = 0;

                        // Only process new slots
                        if current_slot > last_processed_slot {
                            let start_slot = if last_processed_slot == 0 {
                                current_slot.saturating_sub(10) // Start from 10 slots back on first run
                            } else {
                                last_processed_slot + 1
                            };

                            debug!("Processing slots {} to {}", start_slot, current_slot);

                            // Subscribe to logs for our program
                            let logs_config = RpcTransactionLogsConfig {
                                commitment: Some(commitment),
                            };

                            let filter = RpcTransactionLogsFilter::Mentions(vec![program_id.to_string()]);

                            // Get logs for the slot range
                            match client.get_signatures_for_address_with_config(
                                &program_id,
                                solana_client::rpc_config::GetConfirmedSignaturesForAddress2Config {
                                    before: None,
                                    until: None,
                                    limit: Some(100),
                                    commitment: Some(commitment),
                                },
                            ).await {
                                Ok(signatures) => {
                                    for sig_info in signatures {
                                        if sig_info.slot.unwrap_or(0) > last_processed_slot {
                                            if let Ok(signature) = Signature::from_str(&sig_info.signature) {
                                                match client.get_transaction(&signature, solana_client::rpc_config::RpcTransactionConfig {
                                                    encoding: Some(solana_account_decoder::UiTransactionEncoding::Json),
                                                    commitment: Some(commitment),
                                                    max_supported_transaction_version: Some(0),
                                                }).await {
                                                    Ok(confirmed_transaction) => {
                                                        if let Some(transaction) = confirmed_transaction.transaction {
                                                            if let Some(meta) = transaction.meta {
                                                                if let Some(log_messages) = meta.log_messages {
                                                                    // Check if this is a settlement transaction
                                                                    let contains_settlement = log_messages.iter()
                                                                        .any(|log| log.contains("SETTLEMENT_EVENT:"));
                                                                    
                                                                    if contains_settlement {
                                                                        if let Some(instruction) = Self::parse_settlement_event(
                                                                            &SolanaChain {
                                                                                client: client.clone(),
                                                                                config: SolanaConfig {
                                                                                    rpc_url: "".to_string(),
                                                                                    program_id: "".to_string(),
                                                                                    commitment: "confirmed".to_string(),
                                                                                    poll_interval_ms: 1000,
                                                                                    max_retries: 3,
                                                                                },
                                                                                program_id,
                                                                                commitment,
                                                                                event_sender: None,
                                                                            },
                                                                            &log_messages,
                                                                            &sig_info.signature,
                                                                            sig_info.slot.unwrap_or(0),
                                                                            sig_info.block_time,
                                                                        ) {
                                                                            if let Some(sender) = &event_sender {
                                                                                if let Err(e) = sender.send(instruction) {
                                                                                    error!("Failed to send settlement instruction: {}", e);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        debug!("Failed to get transaction {}: {}", sig_info.signature, e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to get signatures for program {}: {}", program_id, e);
                                }
                            }

                            last_processed_slot = current_slot;
                        }
                    }
                    Err(e) => {
                        error!("Failed to get current slot: {}", e);
                        retry_count += 1;
                        
                        if retry_count >= max_retries {
                            error!("Max retries reached for slot polling, backing off");
                            sleep(Duration::from_secs(60)).await;
                            retry_count = 0;
                        } else {
                            sleep(Duration::from_secs(5)).await;
                        }
                    }
                }

                sleep(poll_interval).await;
            }
        });

        Ok(())
    }

    async fn get_settlement_events(
        &self,
        from_slot: Option<u64>,
    ) -> Result<Vec<SettlementInstruction>, SettlementError> {
        info!("Fetching settlement events from slot: {:?}", from_slot);

        let current_slot = self.client.get_slot_with_commitment(self.commitment).await
            .map_err(|e| SettlementError::ChainError(format!("Failed to get current slot: {}", e)))?;

        let start_slot = from_slot.unwrap_or(current_slot.saturating_sub(100));

        let signatures = self.client.get_signatures_for_address_with_config(
            &self.program_id,
            solana_client::rpc_config::GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: None,
                limit: Some(100),
                commitment: Some(self.commitment),
            },
        ).await
        .map_err(|e| SettlementError::ChainError(format!("Failed to get signatures: {}", e)))?;

        let mut instructions = Vec::new();

        for sig_info in signatures {
            if sig_info.slot.unwrap_or(0) >= start_slot {
                if let Ok(signature) = Signature::from_str(&sig_info.signature) {
                    match self.client.get_transaction(&signature, solana_client::rpc_config::RpcTransactionConfig {
                        encoding: Some(solana_account_decoder::UiTransactionEncoding::Json),
                        commitment: Some(self.commitment),
                        max_supported_transaction_version: Some(0),
                    }).await {
                        Ok(confirmed_transaction) => {
                            if let Some(transaction) = confirmed_transaction.transaction {
                                if let Some(meta) = transaction.meta {
                                    if let Some(log_messages) = meta.log_messages {
                                        if let Some(instruction) = self.parse_settlement_event(
                                            &log_messages,
                                            &sig_info.signature,
                                            sig_info.slot.unwrap_or(0),
                                            sig_info.block_time,
                                        ) {
                                            instructions.push(instruction);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to get transaction {}: {}", sig_info.signature, e);
                        }
                    }
                }
            }
        }

        info!("Found {} settlement events", instructions.len());
        Ok(instructions)
    }

    async fn verify_transaction(&self, tx_hash: &TransactionHash) -> Result<bool, SettlementError> {
        let signature = Signature::from_str(&tx_hash.0)
            .map_err(|e| SettlementError::ChainError(format!("Invalid signature: {}", e)))?;

        match self.client.get_signature_status(&signature).await {
            Ok(Some(result)) => Ok(result.is_ok()),
            Ok(None) => Ok(false),
            Err(e) => Err(SettlementError::ChainError(format!("Failed to verify transaction: {}", e))),
        }
    }

    async fn get_latest_slot(&self) -> Result<u64, SettlementError> {
        self.client.get_slot_with_commitment(self.commitment).await
            .map_err(|e| SettlementError::ChainError(format!("Failed to get latest slot: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SolanaConfig;

    #[tokio::test]
    async fn test_solana_chain_creation() {
        let config = SolanaConfig {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: "11111111111111111111111111111112".to_string(),
            commitment: "confirmed".to_string(),
            poll_interval_ms: 1000,
            max_retries: 3,
        };

        let chain = SolanaChain::new(config, None);
        assert!(chain.is_ok());
    }

    #[test]
    fn test_settlement_event_parsing() {
        let config = SolanaConfig {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: "11111111111111111111111111111112".to_string(),
            commitment: "confirmed".to_string(),
            poll_interval_ms: 1000,
            max_retries: 3,
        };

        let chain = SolanaChain::new(config, None).unwrap();
        
        let logs = vec![
            "Program log: Cyrus Protocol Settlement Request".to_string(),
            "Program log: SETTLEMENT_EVENT: {\"aptos_recipient\":\"0x123\",\"amount\":1000000,\"nonce\":42,\"slot\":12345,\"timestamp\":1640995200}".to_string(),
        ];

        let instruction = chain.parse_settlement_event(&logs, "test_signature", 12345, Some(1640995200));
        assert!(instruction.is_some());
        
        let instruction = instruction.unwrap();
        assert_eq!(instruction.amount, 1000000);
        assert_eq!(instruction.nonce, 42);
        assert_eq!(instruction.receiver.0, "0x123");
    }
}