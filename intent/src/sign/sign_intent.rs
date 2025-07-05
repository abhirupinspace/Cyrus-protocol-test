use ed25519_dalek::{SigningKey, Signature, Signer};
use rand::rngs::OsRng;
use crate::types::intent::SettlementIntent;

use sha2::Sha512;

pub fn sign_intent(intent: &SettlementIntent, signing_key: &SigningKey) -> Signature {
    let intent_bytes = serde_json::to_vec(intent).expect("Failed to serialize intent");
    signing_key.sign(&intent_bytes)
}