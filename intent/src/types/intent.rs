use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementIntent {
    pub protocol_version: u8,
    pub intent_id: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub sender: String,
    pub receiver: String,
    pub asset: String,
    pub amount: u64,
    pub nonce: u64,
    pub timestamp: u64,
    pub expiry: u64,
    pub signature: Option<String>,
}
