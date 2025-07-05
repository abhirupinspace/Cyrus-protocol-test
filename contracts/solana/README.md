# Cyrus Protocol Solana Contract

 **Simple settlement instruction emitter for cross-chain settlements**

## What This Contract Does

1. **Simulates a swap** (no actual tokens involved for demo)
2. **Emits settlement events** with structured data
3. **Provides transaction signatures** for the relayer to process
4. **Works with your relayer** to trigger Aptos settlements

## File Structure

```
contracts/solana/
├── Anchor.toml                    # Anchor configuration
├── Cargo.toml                     # Workspace config  
├── package.json                   # Node dependencies
├── programs/
│   └── cyrus-solana/
│       ├── Cargo.toml            # Program dependencies
│       └── src/
│           └── lib.rs            # Main program code
└── tests/
    └── cyrus-solana.ts           # Test file
```

## Quick Setup

### 1. Create Directory Structure
```bash
cd cyrus-protocol
mkdir -p contracts/solana/programs/cyrus-solana/src
mkdir -p contracts/solana/tests
cd contracts/solana
```

### 2. Copy Artifact Files
Copy these artifacts to their respective files:
- **Anchor.toml** → `Anchor.toml`
- **Solana Workspace Cargo.toml** → `Cargo.toml`
- **Program Cargo.toml** → `programs/cyrus-solana/Cargo.toml`
- **Solana Program lib.rs** → `programs/cyrus-solana/src/lib.rs`
- **Solana Test** → `tests/cyrus-solana.ts`
- **package.json** → `package.json`

### 3. Install & Build
```bash
# Install dependencies
yarn install

# Build the program
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### 4. Update Program ID
After deployment, update the program ID in:
- `Anchor.toml` (programs section)
- `programs/cyrus-solana/src/lib.rs` (declare_id! macro)

### 5. Test
```bash
anchor test
```

## Core Functions

### `emit_settlement`
**Simple version** - just emits the settlement event:
```rust
pub fn emit_settlement(
    ctx: Context<RequestSettlement>,
    aptos_recipient: String,  // "0x123..."
    amount_usdc: u64,         // Amount in micro USDC
) -> Result<()>
```

### `request_settlement`  
**Full version** - includes more metadata:
```rust
pub fn request_settlement(
    ctx: Context<RequestSettlement>,
    amount_usdc: u64,        // Amount in micro USDC  
    aptos_recipient: String, // "0x123..."
) -> Result<()>
```

## Event Format

```rust
pub struct SettlementRequested {
    pub source_chain: String,     // "solana"
    pub aptos_recipient: String,  // "0x123..."
    pub amount: u64,              // micro USDC
    pub nonce: u64,               // unique slot number
    pub slot: u64,                // Solana slot
    pub timestamp: u64,           // Unix timestamp
}
```

## Testing

### Run Tests
```bash
anchor test
```

### Manual Testing
```bash
# Call the function directly
anchor run test
```

### Expected Output
```
Testing Cyrus Solana Settlement Emitter
Requesting settlement:
   Amount: 1 USDC
   Aptos Recipient: 0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd
✅ Transaction signature: 4zQ8X9xY2B3vL7mN8oP1rT6uW5eR9tY2xZ3bM4nK7pL
🔗 Explorer: https://explorer.solana.com/tx/4zQ8X9xY2B3vL7mN8oP1rT6uW5eR9tY2xZ3bM4nK7pL?cluster=devnet

Settlement request emitted successfully!
```

## 🔗 Integration with Relayer

### Manual Integration (Current)
1. **Deploy Solana contract**
2. **Call emit_settlement()**
3. **Get transaction signature**
4. **Manually feed to relayer:**
   ```bash
   # Extract from Solana transaction
   TX_HASH="4zQ8X9xY2B3vL7mN8oP1rT6uW5eR9tY2xZ3bM4nK7pL"
   RECEIVER="0xcd63ab17ff17b42a9d5c893cf3be1ceba94243111380ff2ce76f6a6083a090dd"
   AMOUNT="1000000"
   
   # Update relayer main.rs with these values
   let settlement = SettlementInstruction::new(
       TX_HASH.to_string(),
       RECEIVER.to_string(), 
       AMOUNT.parse().unwrap(),
       1,
   );
   ```

### Future: Automatic Integration
Your relayer can be extended to:
1. **Listen for Solana events**
2. **Parse SettlementRequested events**
3. **Auto-extract transaction data**
4. **Submit to Aptos automatically**

## Demo Flow

### Complete Cross-Chain Demo:
1. **Call Solana contract**: `emit_settlement(aptos_recipient, amount)`
2. **Get transaction hash**: From Solana explorer
3. **Feed to relayer**: Update relayer with tx data
4. **Run relayer**: `cargo run --release`
5. **See Aptos settlement**: Check Aptos explorer

### Example Demo Commands:
```bash
# 1. Deploy Solana contract
anchor deploy --provider.cluster devnet

# 2. Run test to emit settlement
anchor test

# 3. Copy transaction hash from output
# TX: 4zQ8X9xY2B3vL7mN8oP1rT6uW5eR9tY2xZ3bM4nK7pL

# 4. Update relayer with this transaction hash
# Edit relayer/src/main.rs with the real TX hash

# 5. Run relayer to process the settlement
cd ../../relayer
cargo run --release
```

## **Complete Architecture**

```
Solana Contract              Relayer                 Aptos Contract
┌─────────────────┐         ┌──────────────┐         ┌─────────────────┐
│ emit_settlement │ ──────▶ │ Settlement   │ ──────▶ │ settle()        │
│                 │  TX Hash │ Processor    │ Aptos TX │                 │
│ SettlementEvent │         │              │         │ Transfer USDC   │
└─────────────────┘         └──────────────┘         └─────────────────┘
      Solana                    Rust CLI                    Aptos
```

