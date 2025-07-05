mod types;
mod settlement_processor;

use crate::{
    types::{SettlementInstruction, RelayerConfig},
    settlement_processor::SettlementProcessor,
};
use anyhow::Result;
use log::{info, error};


#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Cyrus Protocol Demo Settlement Processor");
    info!("======================================");
    
    // Configuration with actual values
    let config = RelayerConfig::new(
        "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string(),
        "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string(),
    );
    
    // Check if configuration is set (can be removed since we're using real addresses)
    if config.contract_address.is_empty() {
        error!("❌ Contract address cannot be empty!");
        std::process::exit(1);
    }
    
    // Create settlement processor
    let processor = SettlementProcessor::new(config);
    
    // Show current vault status
    info!("Current Vault Status:");
    match processor.get_vault_balance().await {
        Ok(balance) => {
            let usdc_balance = balance as f64 / 1_000_000.0;
            info!("   Vault Balance: {} USDC ({} micro USDC)", usdc_balance, balance);
        }
        Err(e) => error!("   Could not get vault balance: {}", e),
    }
    
    match processor.get_total_settled().await {
        Ok(total) => {
            let usdc_total = total as f64 / 1_000_000.0;
            info!("   Total Settled: {} USDC ({} micro USDC)", usdc_total, total);
        }
        Err(e) => error!("   Could not get total settled: {}", e),
    }
    
    // Example settlement - YOU CAN MODIFY THIS
    info!("");
    info!("Processing Example Settlement:");
    
    let settlement = SettlementInstruction::new(
        "example_solana_tx_hash_123456789".to_string(),    // Source transaction hash
        "0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd".to_string(), 
        0.1,  // Amount in USDC
        1,    // Nonce
    );
    
    info!("   Source TX: {}", settlement.source_tx_hash);
    info!("   Receiver: {}", settlement.receiver);
    info!("   Amount: {} USDC", settlement.amount_in_usdc());
    info!("   Nonce: {}", settlement.nonce);
    
    // Process the settlement
    match processor.process_settlement(settlement).await {
        Ok(result) => {
            if result.success {
                info!(" Settlement Successful!");
                if let Some(tx_hash) = result.tx_hash {
                    info!("   Transaction Hash: {}", tx_hash);
                    info!("   Explorer: https://explorer.aptoslabs.com/txn/{}?network=testnet", tx_hash);
                }
            } else {
                error!("❌ Settlement Failed!");
                if let Some(error) = result.error {
                    error!("   Error: {}", error);
                }
            }
        }
        Err(e) => {
            error!("❌ Processing Error: {}", e);
        }
    }
    
    info!("");
    info!("Settlement processing complete!");
    info!("");
    info!(" To process different settlements:");
    info!("   1. Modify the settlement instruction in main.rs");
    info!("   2. Run: cargo run --release");
    info!("");
    info!("Or use this as a library:");
    info!("   let processor = SettlementProcessor::new(config);");
    info!("   let result = processor.process_settlement(instruction).await;");
    
    Ok(())
}

// Helper function to create a settlement from command line args (future use)
#[allow(dead_code)]
fn create_settlement_from_args() -> Option<SettlementInstruction> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() >= 5 {
        let source_tx = args[1].clone();
        let receiver = args[2].clone();
        let amount: f64 = args[3].parse().ok()?;
        let nonce: u64 = args[4].parse().ok()?;
        
        Some(SettlementInstruction::new(source_tx, receiver, amount, nonce))
    } else {
        None
    }
}

// Example function showing how to use as a library
#[allow(dead_code)]
async fn example_library_usage() -> Result<()> {
    // This shows how other code can use the settlement processor
    
    let config = RelayerConfig::new(
        "0x1234...".to_string(),
        "0x5678...".to_string(),
    );
    
    let processor = SettlementProcessor::new(config);
    
    // Create settlement from external data (e.g., Solana event)
    let settlement = SettlementInstruction::new(
        "solana_tx_from_event".to_string(),
        "0xrecipient_from_event".to_string(),
        2.5, // 2.5 USDC
        42,  // nonce from event
    );
    
    // Process it
    let result = processor.process_settlement(settlement).await?;
    
    if result.success {
        println!("Settlement processed: {}", result.tx_hash.unwrap());
    } else {
        println!("Settlement failed: {}", result.error.unwrap());
    }
    
    Ok(())
}