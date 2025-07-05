use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use crate::types::intent::SettlementIntent;

pub fn verify_intent(intent: &SettlementIntent, public_key: &VerifyingKey, sig_bytes: &Vec<u8>) -> bool {
    let intent_bytes = serde_json::to_vec(intent).expect("Failed to serialize intent");

    let sig_array: [u8; 64] = match sig_bytes.clone().try_into() {
        Ok(arr) => arr,
        Err(_) => return false,
    };

    let signature = Signature::from(sig_array);

    public_key.verify(&intent_bytes, &signature).is_ok()
}
