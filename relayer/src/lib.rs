// //! Cyrus Protocol Settlement Processor
// //! 
// //! Core library for processing cross-chain settlements between Solana and Aptos.
// //! 
// //! # Example
// //! 
// //! ```no_run
// //! use cyrus_relayer::{SettlementProcessor, SettlementInstruction, RelayerConfig};
// //! 
// //! #[tokio::main]
// //! async fn main() -> Result<(), Box<dyn std::error::Error>> {
// //!     let config = RelayerConfig::new(
// //!         "0x123...".to_string(),  // contract address
// //!         "0x456...".to_string(),  // vault owner
// //!     );
// //!     
// //!     let processor = SettlementProcessor::new(config);
// //!     
// //!     let settlement = SettlementInstruction::new(
// //!         "solana_tx_hash".to_string(),
// //!         "0xaptos_recipient".to_string(),
// //!         1.5, // 1.5 USDC
// //!         1,   // nonce
// //!     );
// //!     
// //!     let result = processor.process_settlement(settlement).await?;
// //!     
// //!     if result.success {
// //!         println!("Settlement successful: {}", result.tx_hash.unwrap());
// //!     }
// //!     
// //!     Ok(())
// //! }
// //! ```

// mod types;
// mod processor;

// // Re-export public API
// pub use types::{SettlementInstruction, SettlementResult, RelayerConfig};
// pub use processor::SettlementProcessor;

// // Convenience functions

// /// Create a new settlement instruction
// pub fn create_settlement(
//     source_tx_hash: String,
//     receiver: String, 
//     amount_usdc: f64,
//     nonce: u64,
// ) -> SettlementInstruction {
//     SettlementInstruction::new(source_tx_hash, receiver, amount_usdc, nonce)
// }

// /// Create a new settlement processor with configuration
// pub fn create_processor(contract_address: String, vault_owner: String) -> SettlementProcessor {
//     let config = RelayerConfig::new(contract_address, vault_owner);
//     SettlementProcessor::new(config)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
    
//     #[test]
//     fn test_settlement_creation() {
//         let settlement = create_settlement(
//             "test_tx".to_string(),
//             "0x123".to_string(),
//             1.5,
//             42,
//         );
        
//         assert_eq!(settlement.source_tx_hash, "test_tx");
//         assert_eq!(settlement.receiver, "0x123");
//         assert_eq!(settlement.amount, 1_500_000); // 1.5 USDC in micro units
//         assert_eq!(settlement.nonce, 42);
//         assert_eq!(settlement.amount_in_usdc(), 1.5);
//     }
    
//     #[test]
//     fn test_settlement_validation() {
//         let valid = create_settlement(
//             "valid_tx".to_string(),
//             "0x123".to_string(),
//             1.0,
//             1,
//         );
//         assert!(valid.validate().is_ok());
        
//         let invalid_address = create_settlement(
//             "test".to_string(),
//             "invalid_address".to_string(),
//             1.0,
//             1,
//         );
//         assert!(invalid_address.validate().is_err());
        
//         let zero_amount = create_settlement(
//             "test".to_string(),
//             "0x123".to_string(),
//             0.0,
//             1,
//         );
//         assert!(zero_amount.validate().is_err());
//     }
// }