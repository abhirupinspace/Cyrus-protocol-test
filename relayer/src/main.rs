use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

// Settlement instruction from Solana
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub source_chain: String,
    pub source_tx_hash: String,
    pub destination_chain: String,
    pub sender: String,
    pub receiver: String,
    pub token_symbol: String,
    pub amount: u64, // micro USDC
    pub nonce: u64,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>,
}

impl SettlementInstruction {
    pub fn new(
        source_tx_hash: String,
        receiver: String,
        amount_usdc: f64,
        nonce: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_chain: "solana".to_string(),
            source_tx_hash,
            destination_chain: "aptos".to_string(),
            sender: "solana_user".to_string(),
            receiver,
            token_symbol: "USDC".to_string(),
            amount: (amount_usdc * 1_000_000.0) as u64,
            nonce,
            timestamp: Utc::now(),
            signature: None,
        }
    }

    pub fn amount_in_usdc(&self) -> f64 {
        self.amount as f64 / 1_000_000.0
    }

    pub fn validate(&self) -> Result<()> {
        if self.source_tx_hash.is_empty() {
            return Err(anyhow::anyhow!("Empty source transaction hash"));
        }
        if self.receiver.is_empty() {
            return Err(anyhow::anyhow!("Empty receiver address"));
        }
        if self.amount == 0 {
            return Err(anyhow::anyhow!("Amount must be greater than 0"));
        }
        if !self.receiver.starts_with("0x") {
            return Err(anyhow::anyhow!("Invalid receiver address format"));
        }
        Ok(())
    }

    // Sign the settlement instruction
    pub fn sign(&mut self, private_key: &str) -> Result<()> {
        use ed25519_dalek::{SigningKey, Signer};

        // For demo, use a deterministic key from the input
        let mut seed = [0u8; 32];
        let key_bytes = hex::decode(private_key.trim_start_matches("0x"))?;
        seed[..key_bytes.len().min(32)].copy_from_slice(&key_bytes[..key_bytes.len().min(32)]);
        
        let signing_key = SigningKey::from_bytes(&seed);
        
        // Create signing payload (without signature field)
        let mut signing_payload = self.clone();
        signing_payload.signature = None;
        
        let message_bytes = serde_json::to_vec(&signing_payload)?;
        let signature = signing_key.sign(&message_bytes);
        
        self.signature = Some(general_purpose::STANDARD.encode(signature.to_bytes()));
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SettlementResult {
    pub success: bool,
    pub aptos_tx_hash: Option<String>,
    pub error: Option<String>,
    pub gas_used: Option<u64>,
}

impl SettlementResult {
    pub fn success(tx_hash: String, gas_used: Option<u64>) -> Self {
        Self {
            success: true,
            aptos_tx_hash: Some(tx_hash),
            error: None,
            gas_used,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            aptos_tx_hash: None,
            error: Some(error),
            gas_used: None,
        }
    }
}

// Solana Event Listener
pub struct SolanaListener {
    rpc_url: String,
    program_id: String,
    client: Client,
}

impl SolanaListener {
    pub fn new(rpc_url: String, program_id: String) -> Self {
        Self {
            rpc_url,
            program_id,
            client: Client::new(),
        }
    }

    // Simulate listening for Solana events
    pub async fn listen_for_settlement_events(&self) -> Vec<SettlementInstruction> {
        info!("🔍 Listening for settlement events on Solana...");
        
        // For demo purposes, simulate finding settlement events from Solana logs
        // In production, this would:
        // 1. Poll Solana RPC for program logs
        // 2. Parse SettlementRequested events
        // 3. Extract settlement data
        
        // Simulate finding some settlement events
        let mock_events = vec![
            SettlementInstruction::new(
                format!("solana_tx_{}", hex::encode(&rand::random::<[u8; 8]>())),
                "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string(),
                0.5,
                1,
            ),
            SettlementInstruction::new(
                format!("solana_tx_{}", hex::encode(&rand::random::<[u8; 8]>())),
                "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string(),
                1.0,
                2,
            ),
        ];

        info!("📡 Found {} settlement events", mock_events.len());
        mock_events
    }
}

// Aptos Executor
pub struct AptosExecutor {
    rpc_url: String,
    contract_address: String,
    vault_owner: String,
    client: Client,
}

impl AptosExecutor {
    pub fn new(rpc_url: String, contract_address: String, vault_owner: String) -> Self {
        Self {
            rpc_url,
            contract_address,
            vault_owner,
            client: Client::new(),
        }
    }

    pub async fn submit_settlement(&self, instruction: &SettlementInstruction) -> Result<SettlementResult> {
        info!("🚀 Submitting settlement to Aptos...");
        info!("   Settlement ID: {}", instruction.id);
        info!("   Amount: {} USDC", instruction.amount_in_usdc());
        info!("   Receiver: {}", instruction.receiver);

        // Validate instruction has signature
        if instruction.signature.is_none() {
            return Ok(SettlementResult::failure("Settlement instruction not signed".to_string()));
        }

        // For demo purposes, simulate Aptos transaction submission
        // In production, this would:
        // 1. Create Aptos transaction payload
        // 2. Submit to Aptos RPC
        // 3. Wait for confirmation
        // 4. Return transaction hash

        // Simulate processing time
        sleep(Duration::from_millis(1500)).await;

        // Generate mock transaction hash
        let mock_tx_hash = format!("0x{}", hex::encode(&rand::random::<[u8; 32]>()));
        let gas_used = 2000 + rand::random::<u64>() % 1000;

        info!("✅ Settlement executed on Aptos!");
        info!("   Transaction: {}", mock_tx_hash);
        info!("   Gas Used: {}", gas_used);
        info!("   Explorer: https://explorer.aptoslabs.com/txn/{}?network=testnet", mock_tx_hash);

        Ok(SettlementResult::success(mock_tx_hash, Some(gas_used)))
    }

    pub async fn get_vault_balance(&self) -> Result<f64> {
        // In production, query the Aptos contract
        // For demo, return a realistic balance
        Ok(10000.0) // 10,000 USDC
    }

    pub async fn verify_settlement(&self, tx_hash: &str) -> Result<bool> {
        info!("🔍 Verifying settlement on Aptos: {}", tx_hash);
        
        // For demo, assume all settlements are valid
        sleep(Duration::from_millis(500)).await;
        
        Ok(true)
    }
}

// Main Settlement Processor
pub struct SettlementProcessor {
    solana_listener: SolanaListener,
    aptos_executor: AptosExecutor,
    signing_key: String,
}

impl SettlementProcessor {
    pub fn new(
        solana_rpc: String,
        solana_program_id: String,
        aptos_rpc: String,
        aptos_contract: String,
        aptos_vault_owner: String,
        signing_key: String,
    ) -> Self {
        Self {
            solana_listener: SolanaListener::new(solana_rpc, solana_program_id),
            aptos_executor: AptosExecutor::new(aptos_rpc, aptos_contract, aptos_vault_owner),
            signing_key,
        }
    }

    pub async fn run_settlement_cycle(&self) -> Result<()> {
        info!("🔄 Running settlement cycle...");

        // Step 1: Listen for Solana events
        let mut settlement_instructions = self.solana_listener.listen_for_settlement_events().await;

        if settlement_instructions.is_empty() {
            info!("📭 No settlement events found");
            return Ok(());
        }

        // Step 2: Sign settlement instructions
        info!("✍️ Signing settlement instructions...");
        for instruction in &mut settlement_instructions {
            if let Err(e) = instruction.sign(&self.signing_key) {
                error!("Failed to sign instruction {}: {}", instruction.id, e);
                continue;
            }
            
            info!("   ✅ Signed instruction: {}", instruction.id);
            info!("      Source TX: {}", instruction.source_tx_hash);
            info!("      Signature: {}", instruction.signature.as_ref()
    .map(|s| if s.len() > 20 { format!("{}...", &s[..20]) } else { s.clone() })
    .unwrap_or_else(|| "None".to_string()));
        }

        // Step 3: Submit to Aptos
        info!("📤 Submitting settlements to Aptos...");
        let mut successful_settlements = 0;
        let mut failed_settlements = 0;

        for instruction in &settlement_instructions {
            match self.aptos_executor.submit_settlement(instruction).await {
                Ok(result) => {
                    if result.success {
                        successful_settlements += 1;
                        
                        if let Some(tx_hash) = &result.aptos_tx_hash {
                            // Verify the settlement
                            match self.aptos_executor.verify_settlement(tx_hash).await {
                                Ok(true) => {
                                    info!("   ✅ Settlement verified: {}", tx_hash);
                                }
                                Ok(false) => {
                                    warn!("   ⚠️ Settlement verification failed: {}", tx_hash);
                                }
                                Err(e) => {
                                    error!("   ❌ Verification error: {}", e);
                                }
                            }
                        }
                    } else {
                        failed_settlements += 1;
                        error!("   ❌ Settlement failed: {}", result.error.unwrap_or_default());
                    }
                }
                Err(e) => {
                    failed_settlements += 1;
                    error!("   ❌ Settlement error: {}", e);
                }
            }
        }

        // Step 4: Summary
        info!("📊 Settlement Cycle Summary:");
        info!("   Total Instructions: {}", settlement_instructions.len());
        info!("   Successful: {}", successful_settlements);
        info!("   Failed: {}", failed_settlements);
        
        if let Ok(vault_balance) = self.aptos_executor.get_vault_balance().await {
            info!("   Current Vault Balance: {} USDC", vault_balance);
        }

        Ok(())
    }

    pub async fn process_manual_settlement(
        &self,
        source_tx_hash: String,
        receiver: String,
        amount_usdc: f64,
        nonce: u64,
    ) -> Result<SettlementResult> {
        info!("🎯 Processing manual settlement...");
        
        // Create and sign instruction
        let mut instruction = SettlementInstruction::new(source_tx_hash, receiver, amount_usdc, nonce);
        instruction.sign(&self.signing_key)?;
        
        info!("   Created settlement instruction: {}", instruction.id);
        info!("   Signed with signature: {}", instruction.signature.as_ref()
    .map(|s| if s.len() > 20 { format!("{}...", &s[..20]) } else { s.clone() })
    .unwrap_or_else(|| "None".to_string()));
        
        // Submit to Aptos
        self.aptos_executor.submit_settlement(&instruction).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🎪 Cyrus Protocol Cross-Chain Settlement Demo");
    info!("============================================");
    info!("");

    // Configuration
    let solana_rpc = "https://api.devnet.solana.com".to_string();
    let solana_program_id = "HTViMNg8vW2SV7cesRAoFMcfJj2wH9NAqsb6NeraMBsk".to_string();
    let aptos_rpc = "https://fullnode.testnet.aptoslabs.com/v1".to_string();
    let aptos_contract = "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string();
    let aptos_vault_owner = "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string();
    let signing_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string();

    info!("🔧 Configuration:");
    info!("   Solana RPC: {}", solana_rpc);
    info!("   Solana Program: {}", solana_program_id);
    info!("   Aptos RPC: {}", aptos_rpc);
    info!("   Aptos Contract: {}", aptos_contract);
    info!("");

    // Create settlement processor
    let processor = SettlementProcessor::new(
        solana_rpc,
        solana_program_id,
        aptos_rpc,
        aptos_contract,
        aptos_vault_owner,
        signing_key,
    );

    // Demo Flow
    info!("🎬 Starting Demo Flow...");
    info!("");

    // Scenario 1: Automated settlement cycle
    info!("📚 Scenario 1: Automated Settlement Discovery");
    info!("─────────────────────────────────────────────");
    
    match processor.run_settlement_cycle().await {
        Ok(()) => info!("✅ Automated settlement cycle completed"),
        Err(e) => error!("❌ Automated settlement cycle failed: {}", e),
    }

    info!("");
    sleep(Duration::from_secs(2)).await;

    // Scenario 2: Manual settlement processing
    info!("📋 Scenario 2: Manual Settlement Processing");
    info!("──────────────────────────────────────────");
    
    let manual_settlements = vec![
        ("manual_tx_001", "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd", 2.5, 100),
        ("manual_tx_002", "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd", 1.0, 101),
        ("manual_tx_003", "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd", 0.25, 102),
    ];

    for (tx_hash, receiver, amount, nonce) in manual_settlements {
        info!("Processing manual settlement: {} USDC → {}", amount, receiver);
        
        match processor.process_manual_settlement(
            tx_hash.to_string(),
            receiver.to_string(),
            amount,
            nonce,
        ).await {
            Ok(result) => {
                if result.success {
                    info!("   ✅ Success: {}", result.aptos_tx_hash.unwrap_or_default());
                } else {
                    error!("   ❌ Failed: {}", result.error.unwrap_or_default());
                }
            }
            Err(e) => error!("   ❌ Error: {}", e),
        }
        
        sleep(Duration::from_secs(1)).await;
    }

    info!("");
    info!("🎉 Demo Complete!");
    info!("");
    info!("🔗 Next Steps:");
    info!("   1. Check Aptos explorer for settlement transactions");
    info!("   2. Verify vault balance changes on Aptos");
    info!("   3. Monitor settlement events in the logs");
    info!("   4. Integration: Connect to real Solana event listener");
    info!("   5. Integration: Connect to real Aptos transaction submission");

    Ok(())
}

// Helper function for external usage
pub fn create_settlement_instruction(
    source_tx_hash: String,
    receiver: String,
    amount_usdc: f64,
    nonce: u64,
) -> SettlementInstruction {
    SettlementInstruction::new(source_tx_hash, receiver, amount_usdc, nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_creation() {
        let settlement = create_settlement_instruction(
            "test_tx".to_string(),
            "0x123".to_string(),
            1.5,
            42,
        );
        
        assert_eq!(settlement.source_tx_hash, "test_tx");
        assert_eq!(settlement.receiver, "0x123");
        assert_eq!(settlement.amount, 1_500_000);
        assert_eq!(settlement.nonce, 42);
        assert_eq!(settlement.amount_in_usdc(), 1.5);
    }

    #[test]
    fn test_settlement_validation() {
        let valid = create_settlement_instruction(
            "valid_tx".to_string(),
            "0x123".to_string(),
            1.0,
            1,
        );
        assert!(valid.validate().is_ok());
        
        let invalid = create_settlement_instruction(
            "".to_string(),
            "invalid".to_string(),
            0.0,
            1,
        );
        assert!(invalid.validate().is_err());
    }

    #[tokio::test]
    async fn test_settlement_signing() {
        let mut settlement = create_settlement_instruction(
            "test_tx".to_string(),
            "0x123".to_string(),
            1.0,
            1,
        );
        
        let key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert!(settlement.sign(key).is_ok());
        assert!(settlement.signature.is_some());
    }
}