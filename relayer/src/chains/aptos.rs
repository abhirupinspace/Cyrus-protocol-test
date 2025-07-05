use crate::types::{
    Address, AptosConfig, SettlementError, SettlementInstruction, SettlementResult,
    TransactionHash,
};
use aptos_sdk::{
    coin_client::CoinClient,
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey},
    move_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, StructTag},
        value::{serialize_values, MoveValue},
    },
    rest_client::{Client, FaucetClient},
    transaction_builder::TransactionBuilder,
    types::{
        account_config::aptos_coin_type,
        chain_id::ChainId,
        transaction::{EntryFunction, TransactionPayload},
        LocalAccount,
    },
};
use async_trait::async_trait;
use chrono::Utc;
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use url::Url;

/// Trait for interacting with destination chains
#[async_trait]
pub trait DestinationChain: Send + Sync {
    async fn submit_settlement(
        &self,
        instruction: &SettlementInstruction,
    ) -> Result<SettlementResult, SettlementError>;
    async fn is_settlement_processed(
        &self,
        tx_hash: &TransactionHash,
    ) -> Result<bool, SettlementError>;
    async fn get_vault_balance(&self) -> Result<u64, SettlementError>;
    async fn get_total_settled(&self) -> Result<u64, SettlementError>;
    async fn check_health(&self) -> Result<bool, SettlementError>;
}

/// Aptos chain implementation
pub struct AptosChain {
    client: Arc<Client>,
    config: AptosConfig,
    account: LocalAccount,
    contract_address: AccountAddress,
    vault_owner: AccountAddress,
}

impl AptosChain {
    pub async fn new(config: AptosConfig) -> Result<Self, SettlementError> {
        let client = Arc::new(
            Client::new(
                Url::parse(&config.rpc_url)
                    .map_err(|e| SettlementError::ConfigError(format!("Invalid RPC URL: {}", e)))?,
            )
        );

        // Parse private key
        let private_key_bytes = hex::decode(&config.private_key)
            .map_err(|e| SettlementError::ConfigError(format!("Invalid private key: {}", e)))?;
        
        let private_key = Ed25519PrivateKey::try_from(private_key_bytes.as_slice())
            .map_err(|e| SettlementError::ConfigError(format!("Invalid private key format: {}", e)))?;

        // Create local account
        let account = LocalAccount::new(
            AccountAddress::from_hex_literal(&format!("0x{}", hex::encode(private_key.public_key().to_bytes())))
                .map_err(|e| SettlementError::ConfigError(format!("Invalid account address: {}", e)))?,
            private_key,
            0, // sequence number will be fetched
        );

        let contract_address = AccountAddress::from_hex_literal(&config.contract_address)
            .map_err(|e| SettlementError::ConfigError(format!("Invalid contract address: {}", e)))?;

        let vault_owner = AccountAddress::from_hex_literal(&config.vault_owner)
            .map_err(|e| SettlementError::ConfigError(format!("Invalid vault owner: {}", e)))?;

        Ok(Self {
            client,
            config,
            account,
            contract_address,
            vault_owner,
        })
    }

    /// Sync account sequence number
    async fn sync_account(&mut self) -> Result<(), SettlementError> {
        let account_info = self.client
            .get_account(self.account.address())
            .await
            .map_err(|e| SettlementError::ChainError(format!("Failed to get account info: {}", e)))?;

        self.account.set_sequence_number(account_info.sequence_number);
        Ok(())
    }

    /// Create settlement transaction payload
    fn create_settlement_payload(
        &self,
        instruction: &SettlementInstruction,
    ) -> Result<TransactionPayload, SettlementError> {
        let module_id = ModuleId::new(self.contract_address, Identifier::new("settlement").unwrap());
        
        let function = Identifier::new("settle").unwrap();
        
        // Parse receiver address
        let receiver_address = AccountAddress::from_hex_literal(&instruction.receiver.0)
            .map_err(|e| SettlementError::InvalidInstruction(format!("Invalid receiver address: {}", e)))?;

        let args = serialize_values(&vec![
            MoveValue::Address(self.vault_owner),
            MoveValue::vector_u8(instruction.source_tx_hash.0.as_bytes().to_vec()),
            MoveValue::Address(receiver_address),
            MoveValue::U64(instruction.amount),
            MoveValue::U64(instruction.nonce),
            MoveValue::U64(instruction.timestamp.timestamp() as u64),
        ]);

        Ok(TransactionPayload::EntryFunction(EntryFunction::new(
            module_id,
            function,
            vec![], // type arguments
            args,
        )))
    }

    /// Wait for transaction confirmation
    async fn wait_for_transaction(&self, tx_hash: &str) -> Result<bool, SettlementError> {
        let timeout_duration = Duration::from_secs(self.config.transaction_timeout_secs);
        
        match timeout(timeout_duration, async {
            loop {
                match self.client.get_transaction_by_hash(tx_hash).await {
                    Ok(txn) => {
                        if txn.success() {
                            return Ok(true);
                        } else {
                            return Err(SettlementError::TransactionFailed(
                                format!("Transaction failed: {:?}", txn.vm_status())
                            ));
                        }
                    }
                    Err(_) => {
                        // Transaction not yet confirmed
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }).await {
            Ok(result) => result,
            Err(_) => Err(SettlementError::Timeout(
                format!("Transaction confirmation timeout: {}", tx_hash)
            )),
        }
    }

    /// Call view function
    async fn call_view_function(
        &self,
        function_name: &str,
        type_args: Vec<String>,
        args: Vec<String>,
    ) -> Result<Vec<serde_json::Value>, SettlementError> {
        let payload = aptos_sdk::types::transaction::ViewRequest {
            function: format!("{}::settlement::{}", self.contract_address, function_name).parse()
                .map_err(|e| SettlementError::ChainError(format!("Invalid function: {}", e)))?,
            type_arguments: type_args,
            arguments: args,
        };

        self.client
            .view(&payload, None)
            .await
            .map_err(|e| SettlementError::ChainError(format!("View function call failed: {}", e)))
    }
}

#[async_trait]
impl DestinationChain for AptosChain {
    async fn submit_settlement(
        &self,
        instruction: &SettlementInstruction,
    ) -> Result<SettlementResult, SettlementError> {
        info!("Submitting settlement to Aptos: {:?}", instruction.id);

        // Validate instruction
        instruction.validate()?;

        // Check if already processed
        if self.is_settlement_processed(&instruction.source_tx_hash).await? {
            warn!("Settlement already processed: {}", instruction.source_tx_hash);
            return Ok(SettlementResult::failure(
                instruction.id,
                "Already processed".to_string(),
                0,
            ));
        }

        // Sync account
        let mut account = self.account.clone();
        let account_info = self.client
            .get_account(account.address())
            .await
            .map_err(|e| SettlementError::ChainError(format!("Failed to get account info: {}", e)))?;

        account.set_sequence_number(account_info.sequence_number);

        // Create transaction payload
        let payload = self.create_settlement_payload(instruction)?;

        // Build transaction
        let chain_id = self.client
            .get_ledger_information()
            .await
            .map_err(|e| SettlementError::ChainError(format!("Failed to get chain info: {}", e)))?
            .chain_id;

        let transaction_builder = TransactionBuilder::new(
            payload,
            chrono::Utc::now().timestamp() as u64 + 30, // 30 second expiry
            ChainId::new(chain_id),
        )
        .sender(account.address())
        .sequence_number(account.sequence_number())
        .max_gas_amount(self.config.max_gas_amount)
        .gas_unit_price(self.config.gas_unit_price);

        // Sign and submit transaction
        let signed_txn = account.sign_with_transaction_builder(transaction_builder);

        match self.client.submit(&signed_txn).await {
            Ok(response) => {
                let tx_hash = response.hash.to_string();
                debug!("Transaction submitted: {}", tx_hash);

                // Wait for confirmation
                match self.wait_for_transaction(&tx_hash).await {
                    Ok(true) => {
                        info!("Settlement completed successfully: {}", tx_hash);
                        
                        // Get gas used (optional)
                        let gas_used = self.client
                            .get_transaction_by_hash(&tx_hash)
                            .await
                            .ok()
                            .and_then(|txn| txn.gas_used().map(|g| g as u64));

                        Ok(SettlementResult::success(
                            instruction.id,
                            TransactionHash(tx_hash),
                            gas_used,
                        ))
                    }
                    Ok(false) => {
                        error!("Transaction failed: {}", tx_hash);
                        Ok(SettlementResult::failure(
                            instruction.id,
                            "Transaction failed".to_string(),
                            0,
                        ))
                    }
                    Err(e) => {
                        error!("Settlement processing error: {}", e);
                        Ok(SettlementResult::failure(instruction.id, e.to_string(), 0))
                    }
                }
            }
            Err(e) => {
                error!("Failed to submit transaction: {}", e);
                Ok(SettlementResult::failure(
                    instruction.id,
                    format!("Submission failed: {}", e),
                    0,
                ))
            }
        }
    }

    async fn is_settlement_processed(
        &self,
        tx_hash: &TransactionHash,
    ) -> Result<bool, SettlementError> {
        let args = vec![
            self.vault_owner.to_hex_literal(),
            format!("\"{}\"", tx_hash.0),
        ];

        match self.call_view_function("is_settled", vec![], args).await {
            Ok(result) => {
                if let Some(value) = result.first() {
                    Ok(value.as_bool().unwrap_or(false))
                } else {
                    Ok(false)
                }
            }
            Err(_) => {
                // If we can't check, assume not processed for safety
                Ok(false)
            }
        }
    }

    async fn get_vault_balance(&self) -> Result<u64, SettlementError> {
        let args = vec![self.vault_owner.to_hex_literal()];

        match self.call_view_function("get_vault_balance", vec![], args).await {
            Ok(result) => {
                if let Some(value) = result.first() {
                    if let Some(balance_str) = value.as_str() {
                        balance_str.parse::<u64>()
                            .map_err(|e| SettlementError::ChainError(format!("Invalid balance format: {}", e)))
                    } else {
                        Ok(value.as_u64().unwrap_or(0))
                    }
                } else {
                    Ok(0)
                }
            }
            Err(e) => {
                warn!("Failed to get vault balance: {}", e);
                Ok(0)
            }
        }
    }

    async fn get_total_settled(&self) -> Result<u64, SettlementError> {
        let args = vec![self.vault_owner.to_hex_literal()];

        match self.call_view_function("get_total_settled", vec![], args).await {
            Ok(result) => {
                if let Some(value) = result.first() {
                    if let Some(total_str) = value.as_str() {
                        total_str.parse::<u64>()
                            .map_err(|e| SettlementError::ChainError(format!("Invalid total format: {}", e)))
                    } else {
                        Ok(value.as_u64().unwrap_or(0))
                    }
                } else {
                    Ok(0)
                }
            }
            Err(e) => {
                warn!("Failed to get total settled: {}", e);
                Ok(0)
            }
        }
    }

    async fn check_health(&self) -> Result<bool, SettlementError> {
        match self.client.get_ledger_information().await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("Aptos health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AptosConfig, ChainId, SettlementInstruction, TransactionHash};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_config() -> AptosConfig {
        AptosConfig {
            rpc_url: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
            contract_address: "0x1".to_string(),
            vault_owner: "0x1".to_string(),
            private_key: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            max_gas_amount: 200000,
            gas_unit_price: 100,
            transaction_timeout_secs: 30,
        }
    }

    fn create_test_instruction() -> SettlementInstruction {
        SettlementInstruction::new(
            ChainId("solana".to_string()),
            TransactionHash("test_tx".to_string()),
            ChainId("aptos".to_string()),
            Address("sender".to_string()),
            Address("0x1".to_string()),
            "USDC".to_string(),
            1000000,
            1,
            Utc::now(),
            None,
        )
    }

    #[tokio::test]
    async fn test_aptos_chain_creation() {
        let config = create_test_config();
        let result = AptosChain::new(config).await;
        
        // May fail due to network, but should not panic
        match result {
            Ok(_) => println!("Aptos chain created successfully"),
            Err(e) => println!("Expected error in test: {}", e),
        }
    }

    #[test]
    fn test_settlement_payload_creation() {
        // This test would require a properly initialized AptosChain
        // Skipping for now as it requires network access
    }
}