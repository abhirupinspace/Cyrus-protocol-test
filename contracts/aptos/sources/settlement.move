/// This module implements a cross-chain settlement system for USDC transfers from Solana to Aptos.
/// It provides functionality for secure, one-way token transfers with replay protection and
/// proper authorization controls.
///
/// # Features
/// * Vault system for holding USDC tokens
/// * Authorized relayer management
/// * Settlement instruction processing with replay protection
/// * Event emission for settlements
/// * Emergency withdrawal capabilities
/// * Monitoring and view functions
///
/// # Resource Types
/// * `USDC` - Token type representing Circle's USDC on Aptos
/// * `Vault` - Main resource holding USDC balance and settlement state
/// * `SettlementInstruction` - Structure defining cross-chain transfer details
/// * `SettlementEvent` - Event emitted upon successful settlement
///
/// # Role-Based Access Control
/// * Owner - Can initialize vault, deposit/withdraw USDC, and manage relayers
/// * Relayers - Authorized addresses that can submit settlement instructions
/// * Users - Can receive USDC through settlements
///
/// # Error Codes
/// * `E_NOT_OWNER` (1) - Operation requires vault owner authorization
/// * `E_INSUFFICIENT_BALANCE` (2) - Vault has insufficient USDC balance
/// * `E_INVALID_INSTRUCTION` (3) - Settlement instruction is malformed
/// * `E_ALREADY_SETTLED` (4) - Transaction has already been processed
/// * `E_VAULT_NOT_INITIALIZED` (5) - Vault resource doesn't exist
/// * `E_UNAUTHORIZED_RELAYER` (6) - Caller is not an authorized relayer
///
/// # Testing
/// Includes test-only functions for initializing and minting test USDC tokens
 


module cyrus_protocol::settlement {
    use std::string::String;
    use std::signer;
    use std::error;
    use std::vector;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};

    // USDC token type from Circle (Aptos Testnet)
    struct USDC {}

    // Error codes
    const E_NOT_OWNER: u64 = 1;
    const E_INSUFFICIENT_BALANCE: u64 = 2;
    const E_INVALID_INSTRUCTION: u64 = 3;
    const E_ALREADY_SETTLED: u64 = 4;
    const E_VAULT_NOT_INITIALIZED: u64 = 5;
    const E_UNAUTHORIZED_RELAYER: u64 = 6;

    // Settlement instruction from Solana
    struct SettlementInstruction has copy, drop, store {
        source_chain: String,           // "solana"
        source_tx_hash: String,         // Solana transaction hash
        receiver: address,              // Aptos recipient address
        asset: String,                  // "USDC"
        amount: u64,                    // Amount in smallest unit (6 decimals for USDC)
        nonce: u64,                     // Unique nonce for this instruction
        timestamp: u64,                 // Timestamp from source chain
    }

    // Vault resource to hold USDC
    struct Vault has key {
        usdc_balance: Coin<USDC>,
        owner: address,
        authorized_relayers: vector<address>,
        processed_instructions: Table<String, bool>, // tx_hash -> settled
        total_settled: u64,
        settlement_events: EventHandle<SettlementEvent>,
    }

    // Event emitted when settlement completes
    struct SettlementEvent has drop, store {
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        timestamp: u64,
        settlement_time: u64,
    }

    // Initialize the vault (call once after deployment)
    public entry fun initialize_vault(owner: &signer) {
        let owner_addr = signer::address_of(owner);
        
        // Create vault with empty USDC balance
        let vault = Vault {
            usdc_balance: coin::zero<USDC>(),
            owner: owner_addr,
            authorized_relayers: vector::empty<address>(),
            processed_instructions: table::new<String, bool>(),
            total_settled: 0,
            settlement_events: account::new_event_handle<SettlementEvent>(owner),
        };

        move_to(owner, vault);
    }

    // Add USDC to the vault (for testing and liquidity)
    public entry fun deposit_usdc(owner: &signer, amount: u64) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        // Withdraw USDC from owner's account and add to vault
        let deposit_coin = coin::withdraw<USDC>(owner, amount);
        coin::merge(&mut vault.usdc_balance, deposit_coin);
    }

    // Add authorized relayer
    public entry fun add_relayer(owner: &signer, relayer: address) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        vector::push_back(&mut vault.authorized_relayers, relayer);
    }

    // Main settlement function - called by authorized relayers
    public entry fun settle(
        relayer: &signer,
        vault_owner: address,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        _nonce: u64,
        source_timestamp: u64
    ) acquires Vault {
        let relayer_addr = signer::address_of(relayer);
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(vault_owner);
        
        // Check if relayer is authorized (or is owner for testing)
        let is_authorized = vault.owner == relayer_addr || 
                           vector::contains(&vault.authorized_relayers, &relayer_addr);
        assert!(is_authorized, error::permission_denied(E_UNAUTHORIZED_RELAYER));

        // Check if instruction already processed (replay protection)
        assert!(!table::contains(&vault.processed_instructions, source_tx_hash), 
                error::already_exists(E_ALREADY_SETTLED));

        // Validate instruction
        assert!(amount > 0, error::invalid_argument(E_INVALID_INSTRUCTION));
        assert!(coin::value(&vault.usdc_balance) >= amount, 
                error::invalid_state(E_INSUFFICIENT_BALANCE));

        // Mark instruction as processed
        table::add(&mut vault.processed_instructions, source_tx_hash, true);

        // Extract USDC from vault and transfer to receiver
        let settlement_coin = coin::extract(&mut vault.usdc_balance, amount);
        coin::deposit(receiver, settlement_coin);

        // Update statistics
        vault.total_settled = vault.total_settled + amount;

        // Emit settlement event
        let settlement_event = SettlementEvent {
            source_tx_hash,
            receiver,
            amount,
            timestamp: source_timestamp,
            settlement_time: timestamp::now_microseconds(),
        };
        event::emit_event(&mut vault.settlement_events, settlement_event);
    }

    // Helper function to check if instruction was already settled
    public fun is_settled(vault_owner: address, source_tx_hash: String): bool acquires Vault {
        if (!exists<Vault>(vault_owner)) {
            return false
        };
        
        let vault = borrow_global<Vault>(vault_owner);
        table::contains(&vault.processed_instructions, source_tx_hash)
    }

    // View functions for monitoring
    public fun get_vault_balance(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        coin::value(&vault.usdc_balance)
    }

    public fun get_total_settled(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        vault.total_settled
    }

    public fun is_authorized_relayer(vault_owner: address, relayer: address): bool acquires Vault {
        if (!exists<Vault>(vault_owner)) {
            return false
        };
        
        let vault = borrow_global<Vault>(vault_owner);
        vault.owner == relayer || vector::contains(&vault.authorized_relayers, &relayer)
    }

    // Emergency functions
    public entry fun emergency_withdraw(owner: &signer, amount: u64) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        let withdraw_coin = coin::extract(&mut vault.usdc_balance, amount);
        coin::deposit(owner_addr, withdraw_coin);
    }

    #[test_only]
    use aptos_framework::coin::MintCapability;
    #[test_only]
    use std::string;
    
    #[test_only]
    struct TestCapabilities has key {
        mint_cap: MintCapability<USDC>,
    }

    #[test_only]
    public fun init_usdc_for_test(admin: &signer) {
        // Initialize USDC coin for testing
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<USDC>(
            admin,
            string::utf8(b"USD Coin"),
            string::utf8(b"USDC"),
            6, // 6 decimal places
            false,
        );

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        
        move_to(admin, TestCapabilities { mint_cap });
    }

    #[test_only]
    public fun mint_usdc_for_test(admin: &signer, to: address, amount: u64) acquires TestCapabilities {
        let caps = borrow_global<TestCapabilities>(signer::address_of(admin));
        let coins = coin::mint(amount, &caps.mint_cap);
        coin::deposit(to, coins);
    }
}