use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;
use intent::types::intent::SettlementIntent;
use base64::{engine::general_purpose, Engine};
use serde_json;

#[test]
fn test_sign_and_verify_intent() {
    // Manually generate a 32-byte seed
    let mut rng = OsRng;
    let mut seed = [0u8; 32];
    rng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    let mut intent = SettlementIntent {
        protocol_version: 1,
        intent_id: "intent-001".to_string(),
        source_chain: "Solana".to_string(),
        destination_chain: "Ethereum".to_string(),
        asset: "SOL".to_string(),
        sender: "Alice".to_string(),
        receiver: "Bob".to_string(),
        amount: 100,
        expiry: 9999999999,
        nonce: 1,
        timestamp: 1720000000,
        signature: None,
    };

    // Serialize and sign
    let message_bytes = serde_json::to_vec(&intent).unwrap();
    let signature: Signature = signing_key.sign(&message_bytes);
    let sig_vec = signature.to_bytes();

    // Encode signature properly using new base64 API
    let sig_b64 = general_purpose::STANDARD.encode(sig_vec);
    intent.signature = Some(sig_b64);

    // Verification
    let verified = verifying_key.verify(&message_bytes, &signature).is_ok();
    assert!(verified, "Signature verification failed");
}
