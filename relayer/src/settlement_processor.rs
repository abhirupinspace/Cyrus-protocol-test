use crate::types::{SettlementInstruction, SettlementResult, RelayerConfig};
use anyhow::Result;
use log::{info, error, debug};
use tokio::process::Command as AsyncCommand;

/// Core settlement processor
pub struct SettlementProcessor {
    config: RelayerConfig,
}

impl SettlementProcessor {
    /// Create new settlement processor
    pub fn new(config: RelayerConfig) -> Self {
        Self { config }
    }
    
    /// Process a single settlement instruction
    pub async fn process_settlement(&self, instruction: SettlementInstruction) -> Result<SettlementResult> {
        info!("Processing settlement: {} -> {} ({} USDC)", 
              instruction.source_tx_hash,
              instruction.receiver, 
              instruction.amount_in_usdc());
        
        // Validate instruction
        if let Err(e) = instruction.validate() {
            error!("Invalid instruction: {}", e);
            return Ok(SettlementResult::error(e));
        }
        
        // Check if already processed
        if self.is_already_processed(&instruction.source_tx_hash).await? {
            info!("Settlement already processed: {}", instruction.source_tx_hash);
            return Ok(SettlementResult::error("Already processed".to_string()));
        }
        
        // Submit to Aptos
        self.submit_to_aptos(instruction).await
    }
    
    /// Submit settlement to Aptos using CLI
    async fn submit_to_aptos(&self, instruction: SettlementInstruction) -> Result<SettlementResult> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros() as u64;
        
        info!("Submitting to Aptos contract...");
        
        let mut cmd = AsyncCommand::new("aptos");
        cmd.arg("move")
           .arg("run")
           .arg("--function-id")
           .arg(format!("{}::settlement::settle", self.config.contract_address))
           .arg("--args")
           .arg(format!("address:{}", self.config.vault_owner))
           .arg(format!("string:{}", instruction.source_tx_hash))
           .arg(format!("address:{}", instruction.receiver))
           .arg(format!("u64:{}", instruction.amount))
           .arg(format!("u64:{}", instruction.nonce))
           .arg(format!("u64:{}", timestamp))
           .arg("--max-gas")  // Changed from --max-gas-amount to --max-gas
           .arg("200000")
           .arg("--assume-yes");
        
        debug!("Executing: {:?}", cmd);
        
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if output.status.success() {
            // Extract transaction hash
            let tx_hash = self.extract_tx_hash(&stdout).unwrap_or("unknown".to_string());
            info!("✅ Settlement successful: {}", tx_hash);
            Ok(SettlementResult::success(tx_hash))
        } else {
            let error_msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
            error!("❌ Settlement failed: {}", error_msg);
            Ok(SettlementResult::error(error_msg))
        }
    }
    
    /// Check if settlement already processed
    async fn is_already_processed(&self, source_tx_hash: &str) -> Result<bool> {
        let mut cmd = AsyncCommand::new("aptos");
        cmd.arg("move")
           .arg("view")
           .arg("--function-id")
           .arg(format!("{}::settlement::is_settled", self.config.contract_address))
           .arg("--args")
           .arg(format!("address:{}", self.config.vault_owner))
           .arg(format!("string:{}", source_tx_hash));
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains("true"))
        } else {
            // If we can't check, assume not processed
            Ok(false)
        }
    }
    
    /// Extract transaction hash from Aptos CLI output
    fn extract_tx_hash(&self, output: &str) -> Option<String> {
        // Look for transaction hash in various formats
        for line in output.lines() {
            if line.contains("Transaction submitted:") || line.contains("transaction_hash") {
                if let Some(start) = line.find("0x") {
                    let rest = &line[start..];
                    if let Some(end) = rest.find(|c: char| !c.is_ascii_hexdigit() && c != 'x') {
                        if end > 10 { // Reasonable hash length
                            return Some(rest[..end].to_string());
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Get vault balance
    pub async fn get_vault_balance(&self) -> Result<u64> {
        let mut cmd = AsyncCommand::new("aptos");
        cmd.arg("move")
           .arg("view")
           .arg("--function-id")
           .arg(format!("{}::settlement::get_vault_balance", self.config.contract_address))
           .arg("--args")
           .arg(format!("address:{}", self.config.vault_owner));
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(self.extract_number(&stdout).unwrap_or(0))
        } else {
            Ok(0)
        }
    }
    
    /// Get total settled amount
    pub async fn get_total_settled(&self) -> Result<u64> {
        let mut cmd = AsyncCommand::new("aptos");
        cmd.arg("move")
           .arg("view")
           .arg("--function-id")
           .arg(format!("{}::settlement::get_total_settled", self.config.contract_address))
           .arg("--args")
           .arg(format!("address:{}", self.config.vault_owner));
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(self.extract_number(&stdout).unwrap_or(0))
        } else {
            Ok(0)
        }
    }
    
    /// Extract number from CLI output
    fn extract_number(&self, output: &str) -> Option<u64> {
        // Look for numbers in brackets, quotes, or plain text
        for line in output.lines() {
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    let content = &line[start + 1..end];
                    if let Ok(num) = content.trim_matches('"').parse::<u64>() {
                        return Some(num);
                    }
                }
            }
            
            if let Some(start) = line.find('"') {
                let rest = &line[start + 1..];
                if let Some(end) = rest.find('"') {
                    if let Ok(num) = rest[..end].parse::<u64>() {
                        return Some(num);
                    }
                }
            }
            
            // Try parsing the whole line as a number
            if let Ok(num) = line.trim().parse::<u64>() {
                return Some(num);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tx_hash_extraction() {
        let processor = SettlementProcessor::new(RelayerConfig::new(
            "0x123".to_string(),
            "0x456".to_string(),
        ));
        
        let output = "Transaction submitted: https://explorer.aptoslabs.com/txn/0x1234567890abcdef?network=testnet";
        assert!(processor.extract_tx_hash(output).is_some());
    }
    
    #[test]
    fn test_number_extraction() {
        let processor = SettlementProcessor::new(RelayerConfig::new(
            "0x123".to_string(),
            "0x456".to_string(),
        ));
        
        assert_eq!(processor.extract_number(r#"["1000000"]"#), Some(1000000));
        assert_eq!(processor.extract_number(r#""500000""#), Some(500000));
        assert_eq!(processor.extract_number("1500000"), Some(1500000));
    }
}