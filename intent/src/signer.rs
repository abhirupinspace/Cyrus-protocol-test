use crate::types::intent::SettlementIntent;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use base64::{engine::general_purpose, Engine as _};
use serde_json;

pub fn sign_intent(intent: &SettlementIntent, key: &SigningKey) -> String {
    let mut temp = intent.clone();
    temp.signature = None;
    let data = serde_json::to_vec(&temp).unwrap();
    let sig = key.sign(&data);
    general_purpose::STANDARD.encode(sig.to_bytes())
}

pub fn verify_intent(intent: &SettlementIntent, pubkey: &VerifyingKey) -> bool {
    let mut temp = intent.clone();
    let Some(sig_b64) = &temp.signature else {
        return false;
    };

    temp.signature = None;
    let data = serde_json::to_vec(&temp).unwrap();
    let sig_bytes = general_purpose::STANDARD.decode(sig_b64).ok()?;
    let sig = Signature::from_bytes(&sig_bytes.try_into().ok()?).ok()?;

    pubkey.verify(&data, &sig).is_ok()
}
