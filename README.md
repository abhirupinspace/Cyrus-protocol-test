# Cyrus Protocol

A cross-chain settlement protocol enabling seamless asset transfers between Solana and Aptos blockchains through a relayer-based architecture.

## Overview

Cyrus Protocol consists of three main components:

1. **Smart Contracts** - Settlement logic on Aptos and Solana
2. **Intent System** - Cryptographic signing and verification of settlement intents  
3. **Relayer Network** - Off-chain infrastructure for cross-chain communication

## Architecture

![alt text](image.png)

## Components

###  Smart Contracts

#### Aptos Settlement Contract (`contracts/aptos/`)

Move-based settlement contract providing:

- **Vault Management** - USDC custody and balance tracking
- **Relayer Authorization** - Permission system for settlement processors
- **Settlement Execution** - Cross-chain USDC transfers
- **Replay Protection** - Prevents duplicate transaction processing
- **Emergency Controls** - Owner-controlled withdrawal mechanisms

**Key Functions:**
```move
public entry fun initialize_vault(owner: &signer)
public entry fun settle(relayer: &signer, vault_owner: address, ...)
public entry fun add_relayer(owner: &signer, relayer: address)
```

#### Solana Event Emitter (`contracts/solana/`)

Anchor program for settlement event emission:

- **Settlement Events** - Structured data for relayer consumption
- **Cross-chain Metadata** - Aptos recipient and amount information
- **Nonce Generation** - Unique identifiers using Solana slots

**Key Functions:**
```rust
pub fn emit_settlement(ctx: Context<RequestSettlement>, aptos_recipient: String, amount_usdc: u64)
pub fn request_settlement(ctx: Context<RequestSettlement>, amount_usdc: u64, aptos_recipient: String)
```

### Intent System (`intent/`)

Rust CLI for cryptographic intent management:

**Features:**
- Ed25519 signature generation and verification
- JSON-based intent serialization
- Cross-chain settlement intent structure
- Base64 signature encoding

**Usage:**
```bash
# Sign an intent
cargo run -- sign -i unsigned_intent.json -o signed_intent.json

# Verify a signed intent
cargo run -- verify -i signed_intent.json
```

**Intent Structure:**
```json
{
  "protocol_version": 1,
  "intent_id": "intent-001",
  "source_chain": "solana",
  "destination_chain": "aptos",
  "sender": "sender-address",
  "receiver": "receiver-address", 
  "asset": "USDC",
  "amount": 1000000,
  "nonce": 42,
  "timestamp": 1720123400,
  "expiry": 1720126400,
  "signature": "base64-encoded-signature"
}
```

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 16+ 
- Anchor CLI
- Aptos CLI
- Move compiler

### Installation

```bash
# Clone repository
git clone <repository-url>
cd cyrus-protocol

# Install Solana dependencies
cd contracts/solana
yarn install

# Build Aptos contracts
cd ../aptos
aptos move compile

# Build intent system
cd ../../intent
cargo build --release
```

### Running Tests

```bash
# Aptos contract tests
cd contracts/aptos
aptos move test

# Solana contract tests  
cd ../solana
anchor test

# Intent system tests
cd ../../intent
cargo test
```

## Development Workflow

### 1. Deploy Contracts

**Aptos:**
```bash
cd contracts/aptos
aptos move publish --package-dir .
```

**Solana:**
```bash
cd contracts/solana
anchor build
anchor deploy
```

### 2. Create Settlement Intent

```bash
cd intent
cargo run -- sign -i examples/unsigned_intent.json -o signed_intent.json
```

### 3. Process Cross-Chain Settlement

The relayer network monitors Solana events and processes settlement intents on Aptos.

## File Structure

```
cyrus-protocol/
├── contracts/
│   ├── aptos/                    # Move contracts
│   │   ├── sources/
│   │   │   └── settlement.move   # Main settlement logic
│   │   └── tests/
│   └── solana/                   # Anchor programs
│       ├── programs/cyrus-solana/
│       │   └── src/lib.rs        # Event emission
│       └── tests/
├── intent/                       # Intent signing system
│   ├── src/
│   │   ├── types/intent.rs       # Intent data structures
│   │   ├── sign/sign_intent.rs   # Signature generation
│   │   └── verify/verify_intent.rs # Signature verification
│   └── examples/
├── relayer/
    ├── src/
│   ├── main.rs              # Demo application entry point
│   ├── lib.rs               # Library interface
│   ├── types.rs             # Core types and definitions
│   ├── settlement_processor.rs # Main processing logic
│   ├── monitor.rs           # Monitoring and API server
│   └── chains/
│       ├── solana.rs        # Solana chain interface
│       └── aptos.rs         # Aptos chain interface
├── config/
│   └── config.rs            # Configuration management
├── Cargo.toml               # Dependencies and metadata
└── README.md
```
# Cyrus Protocol Cross-Chain Settlement Relayer

A high-performance, production-ready relayer service that processes cross-chain settlements between Solana and Aptos networks for the Cyrus Protocol.

## Overview

The Cyrus Relayer is a critical infrastructure component that:

- **Monitors** Solana for settlement events from the Cyrus Protocol
- **Processes** settlement instructions with retry logic and concurrency controls
- **Executes** corresponding settlements on Aptos
- **Provides** comprehensive monitoring, metrics, and health checks
- **Ensures** reliable cross-chain value transfer with fault tolerance


### Components

- **Solana Chain Interface** (`src/chains/solana.rs`) - Event listening and transaction verification
- **Aptos Chain Interface** (`src/chains/aptos.rs`) - Settlement execution and balance queries
- **Settlement Processor** (`src/settlement_processor.rs`) - Core orchestration logic
- **Database Layer** - Settlement tracking and state management (SQLite/PostgreSQL)
- **Monitoring Server** (`src/monitor.rs`) - Metrics, health checks, and API endpoints
- **Configuration Management** (`config/config.rs`) - Multi-source configuration loading

## Features

- ✅ **Real-time Event Processing** - Monitors Solana program logs in real-time
- ✅ **Concurrent Processing** - Configurable concurrent settlement limits
- ✅ **Retry Logic** - Exponential backoff with configurable retry attempts
- ✅ **Health Monitoring** - Comprehensive health checks for all components
- ✅ **Prometheus Metrics** - Production-ready metrics and monitoring
- ✅ **API Endpoints** - RESTful API for status and management
- ✅ **Fault Tolerance** - Graceful error handling and recovery
- ✅ **Database Persistence** - Settlement state tracking and history
- ✅ **Configuration Flexibility** - Environment variables, CLI args, and config files

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- Access to Solana RPC endpoint
- Access to Aptos RPC endpoint
- Database (SQLite for development, PostgreSQL for production)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd relayer

# Build the relayer
cargo build --release

# Run with default demo configuration
cargo run
```

### Configuration

Create a configuration file `config.toml`:

```toml
[solana]
rpc_url = "https://api.devnet.solana.com"
program_id = "YOUR_SOLANA_PROGRAM_ID"
commitment = "confirmed"
poll_interval_ms = 1000

[aptos]
rpc_url = "https://fullnode.testnet.aptoslabs.com/v1"
contract_address = "0xYOUR_CONTRACT_ADDRESS"
vault_owner = "0xYOUR_VAULT_OWNER"
private_key = "0xYOUR_PRIVATE_KEY"
max_gas_amount = 200000

[processing]
max_concurrent_settlements = 10
retry_attempts = 3
retry_delay_seconds = 5

[monitoring]
metrics_port = 9090
health_check_port = 8080
log_level = "info"

[database]
url = "sqlite:./cyrus-relayer.db"
max_connections = 10
```

### Running

```bash
# Run with config file
cargo run -- --config config.toml

# Run with environment variables
SOLANA_RPC_URL="..." APTOS_RPC_URL="..." cargo run

# Run with CLI arguments
cargo run -- --solana-rpc "..." --aptos-rpc "..." --log-level debug
```

## API Endpoints

The relayer exposes several HTTP endpoints for monitoring and management:

### Health Checks
- `GET /health` - Simple health check (200/503)
- `GET /api/v1/health` - Detailed component health status

### Metrics
- `GET /metrics` - Prometheus metrics format
- `GET /api/v1/metrics` - JSON metrics with statistics
- `GET /api/v1/statistics` - Database statistics

### Management
- `GET /api/v1/status` - Relayer status and performance
- `GET /api/v1/settlements` - Recent settlements list
- `GET /api/v1/settlements/{id}` - Settlement details

### Example API Response

```json
{
  "success": true,
  "data": {
    "metrics": {
      "total_settlements_processed": 1542,
      "successful_settlements": 1538,
      "failed_settlements": 4,
      "pending_settlements": 2,
      "average_processing_time_ms": 2340.5,
      "uptime_seconds": 86400,
      "vault_balance_usdc": 50000.0
    },
    "health": {
      "database": true,
      "solana_chain": true,
      "aptos_chain": true
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## Configuration Options

### Environment Variables

All configuration can be overridden with environment variables:

```bash
# Solana configuration
CYRUS_SOLANA__RPC_URL="https://api.mainnet-beta.solana.com"
CYRUS_SOLANA__PROGRAM_ID="YOUR_PROGRAM_ID"
CYRUS_SOLANA__COMMITMENT="finalized"

# Aptos configuration  
CYRUS_APTOS__RPC_URL="https://fullnode.mainnet.aptoslabs.com/v1"
CYRUS_APTOS__CONTRACT_ADDRESS="0x..."
CYRUS_APTOS__PRIVATE_KEY="0x..."

# Processing configuration
CYRUS_PROCESSING__MAX_CONCURRENT_SETTLEMENTS=20
CYRUS_PROCESSING__RETRY_ATTEMPTS=5

# Database
DATABASE_URL="postgresql://user:pass@localhost/cyrus"

# Monitoring
METRICS_PORT=9090
LOG_LEVEL=info
```

### CLI Arguments

```bash
cargo run -- \
  --config /path/to/config.toml \
  --solana-rpc "https://api.devnet.solana.com" \
  --aptos-rpc "https://fullnode.testnet.aptoslabs.com/v1" \
  --database-url "sqlite:./relayer.db" \
  --metrics-port 9090 \
  --log-level debug
```


### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test types
cargo test settlement_processor

# Run integration tests
cargo test --test integration

# Test with verbose output
cargo test -- --nocapture
```

## Production Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cyrus-relayer /usr/local/bin/
EXPOSE 9090 8080
CMD ["cyrus-relayer"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cyrus-relayer
spec:
  replicas: 1
  selector:
    matchLabels:
      app: cyrus-relayer
  template:
    metadata:
      labels:
        app: cyrus-relayer
    spec:
      containers:
      - name: relayer
        image: cyrus/relayer:latest
        ports:
        - containerPort: 9090
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: cyrus-secrets
              key: database-url
        - name: CYRUS_APTOS__PRIVATE_KEY
          valueFrom:
            secretKeyRef:
              name: cyrus-secrets
              key: aptos-private-key
        resources:
          requests:
            cpu: 100m
            memory: 256Mi
          limits:
            cpu: 500m
            memory: 512Mi
```

### Monitoring Setup

The relayer exposes Prometheus metrics on `/metrics`. Configure your monitoring stack:

```yaml
# prometheus.yml
scrape_configs:
- job_name: 'cyrus-relayer'
  static_configs:
  - targets: ['relayer:9090']
  scrape_interval: 15s
  metrics_path: /metrics
```

### Key Metrics to Monitor

- `cyrus_settlements_total` - Total settlements processed
- `cyrus_settlements_successful_total` - Successful settlements
- `cyrus_settlements_failed_total` - Failed settlements
- `cyrus_settlement_duration_seconds` - Processing time distribution
- `cyrus_vault_balance_usdc` - Current vault balance
- `cyrus_pending_settlements` - Pending settlement count

## Security Considerations

⚠️ **Important Security Notes:**

1. **Private Key Management** - Never commit private keys to version control
2. **Environment Isolation** - Use separate keys for development/production
3. **Network Security** - Restrict RPC access and use TLS
4. **Monitoring** - Monitor for unusual settlement patterns
5. **Access Control** - Limit access to metrics and management endpoints

### Recommended Security Practices

```bash
# Use environment variables for sensitive data
export CYRUS_APTOS__PRIVATE_KEY="$(cat /secure/path/to/key)"

# Use secrets management in production
kubectl create secret generic cyrus-secrets \
  --from-literal=aptos-private-key="0x..." \
  --from-literal=database-url="postgresql://..."

# Enable TLS for RPC endpoints
CYRUS_APTOS__RPC_URL="https://secure-rpc.example.com"
```

## Troubleshooting

### Common Issues

**Connection Errors**
```bash
# Check RPC endpoints
curl -X POST https://api.devnet.solana.com -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Check Aptos connectivity  
curl https://fullnode.testnet.aptoslabs.com/v1/
```

**Configuration Issues**
```bash
# Validate configuration
cargo run -- --config config.toml --validate

# Check environment variables
env | grep CYRUS_
```

**Database Issues**
```bash
# Check database connectivity
sqlite3 cyrus-relayer.db ".tables"

# Reset database (development only)
rm cyrus-relayer.db && cargo run
```

### Debugging

```bash
# Enable detailed logging
RUST_LOG=cyrus_relayer=debug,solana_client=debug cargo run

# Enable network tracing
RUST_LOG=trace cargo run

# Specific component debugging
RUST_LOG=cyrus_relayer::settlement_processor=debug cargo run
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Standards

- Run `cargo fmt` before committing
- Ensure all tests pass (`cargo test`)
- Add tests for new functionality
- Update documentation for API changes
- Follow Rust naming conventions

## License

This project is licensed under the [Apache License 2.0](LICENSE).

## Support

- **Documentation**: [docs.cyrusprotocol.com](https://docs.cyrusprotocol.com)
- **Discord**: [discord.gg/cyrus](https://discord.gg/cyrus)
- **Issues**: [GitHub Issues](https://github.com/cyrus-protocol/relayer/issues)
- **Security**: security@cyrusprotocol.com

## Security Considerations

- **Replay Protection** - All transactions use unique nonces
- **Relayer Authorization** - Only approved relayers can execute settlements
- **Signature Verification** - All intents must be cryptographically signed
- **Emergency Controls** - Vault owners retain emergency withdrawal capabilities
- **Amount Validation** - All transfers validated for sufficient balances

