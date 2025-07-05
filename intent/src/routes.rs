use axum::{
    Json,
    extract::State,
    routing::{get, post},
    Router
};
use std::sync::Arc;
use crate::signer::{sign_intent, verify_intent};
use crate::types::intent::SettlementIntent;
use ed25519_dalek::{SigningKey, VerifyingKey};

#[derive(Clone)]
pub struct AppState {
    pub signing_key: Arc<SigningKey>,
    pub verifying_key: VerifyingKey,
}

pub async fn health() -> &'static str {
    "Cyrus Protocol API OK"
}

pub async fn sign(
    State(state): State<AppState>,
    Json(mut intent): Json<SettlementIntent>,
) -> Json<SettlementIntent> {
    let sig = sign_intent(&intent, &state.signing_key);
    intent.signature = Some(sig);
    Json(intent)
}

pub async fn verify(
    State(state): State<AppState>,
    Json(intent): Json<SettlementIntent>,
) -> String {
    let ok = verify_intent(&intent, &state.verifying_key);
    if ok {
        "✅ Signature is VALID".into()
    } else {
        "❌ Signature is INVALID".into()
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sign", post(sign))
        .route("/verify", post(verify))
        .with_state(state)
}
