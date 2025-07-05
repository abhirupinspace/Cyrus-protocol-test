use intent::types::intent::SettlementIntent;
use intent::sign::sign_intent::sign_intent;
use intent::verify::verify_intent::verify_intent;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use base64::{engine::general_purpose, Engine as _};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use rand::rngs::OsRng;
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "Cyrus CLI")]
#[command(about = "Sign or verify cross-chain settlement intents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sign an unsigned intent JSON
    Sign {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Verify a signed intent JSON
    Verify {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Sign { input, output } => {
            let raw = fs::read_to_string(input).expect("Failed to read input file");
            let mut intent: SettlementIntent = serde_json::from_str(&raw).expect("Invalid JSON");

            // Inject current timestamp if zero
            if intent.timestamp == 0 {
                intent.timestamp = current_unix_timestamp();
            }

            // Generate ephemeral signing key
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);

            // Sign
            let signature = sign_intent(&intent, &signing_key);
            intent.signature = Some(general_purpose::STANDARD.encode(signature.to_bytes()));

            let signed_json = serde_json::to_string_pretty(&intent).unwrap();
            fs::write(output, signed_json).expect("Failed to write output file");
            println!("✅ Intent signed and saved to {:?}", output);
        }
        Commands::Verify { input } => {
            let raw = fs::read_to_string(input).expect("Failed to read input file");
            let signed: SettlementIntent = serde_json::from_str(&raw).expect("Invalid JSON");

            // Generate verifying key (for test; replace with known key in prod)
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);
            let verifying_key: VerifyingKey = signing_key.verifying_key();

            // Strip signature
            let sig_b64 = signed.signature.clone().expect("No signature in intent");
            let sig_bytes = general_purpose::STANDARD.decode(sig_b64).expect("Invalid base64");

            let mut intent_to_verify = signed.clone();
            intent_to_verify.signature = None;

            let valid = verify_intent(&intent_to_verify, &verifying_key, &sig_bytes);
            println!("🔍 Signature is {}", if valid { "✅ VALID" } else { "❌ INVALID" });
        }
    }
}
