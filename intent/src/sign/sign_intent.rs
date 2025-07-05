use crate::types::intent::SettlementIntent;
use ed25519_dalek::{Signature, SigningKey, Signer};
use serde_json;

pub fn sign_intent(intent: &SettlementIntent, signing_key: &SigningKey) -> Signature {
    // Clone and strip signature to ensure a clean signing payload
    let mut stripped_intent = intent.clone();
    stripped_intent.signature = None;

    // Serialize intent to bytes
    let message_bytes = serde_json::to_vec(&stripped_intent)
        .expect("Failed to serialize intent for signing");

    // Sign and return signature
    signing_key.sign(&message_bytes)
}
