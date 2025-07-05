mod types {
    pub mod intent;
}

mod signer;
mod routes;

use axum::Server;
use routes::{routes, AppState};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Cyrus Protocol Signing API is running at http://localhost:3000");

    // Generate keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    // Build app state
    let state = AppState {
        signing_key: Arc::new(signing_key),
        verifying_key,
    };

    // Run server
    let app = routes(state);
    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
