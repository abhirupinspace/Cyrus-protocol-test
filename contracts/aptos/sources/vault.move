module cyrus_protocol::vault {
    use std::string::{Self, String};
    use std::signer;
    use std::error;
    use std::vector;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use cyrus_protocol::types::{Self, VaultStats};
    use cyrus_protocol::events;
    use cyrus_protocol::config;

    // === Circle USDC Integration ===
    // Circle USDC on Aptos: 0x69091fbab5f7d635ee7ac5098cf0c1efbe31d68fec0f2cd565e8d168daf52832::usdc::USDC
    struct USDC has copy, drop, store {}

    // === Error Codes ===
    const E_NOT_OWNER: u64 = 1;
    const E_INSUFFICIENT_BALANCE: u64 = 2;
    const E_INVALID_AMOUNT: u64 = 3;
    const E_VAULT_NOT_INITIALIZED: u64 = 4;
    const E_VAULT_ALREADY_EXISTS: u64 = 5;
    const E_SETTLEMENT_ALREADY_PROCESSED: u64 = 6;
    const E_UNAUTHORIZED_ACCESS: u64 = 7;

    /// Main vault structure for managing USDC settlements
    struct Vault has key {
        owner: address,
        usdc_balance: Coin<USDC>,
        processed_settlements: Table<String, bool>, // tx_hash -> processed
        total_deposited: u64,
        total_settled: u64,
        total_fees_collected: u64,
        settlement_count: u64,
        created_at: u64,
        last_settlement_at: u64,
        is_active: bool,
    }

    /// Vault metadata for querying
    struct VaultMetadata has copy, drop, store {
        owner: address,
        total_balance: u64,
        total_deposited: u64,
        total_settled: u64,
        settlement_count: u64,
        created_at: u64,
        last_activity: u64,
        is_active: bool,
    }

    // === Vault Management ===

    /// Initialize a new USDC vault
    public entry fun initialize_vault(owner: &signer) {
        let owner_addr = signer::address_of(owner);
        
        // Check if vault already exists
        assert!(!exists<Vault>(owner_addr), error::already_exists(E_VAULT_ALREADY_EXISTS));
        
        // Initialize events if not already done
        events::initialize_events(owner);
        
        let vault = Vault {
            owner: owner_addr,
            usdc_balance: coin::zero<USDC>(),
            processed_settlements: table::new<String, bool>(),
            total_deposited: 0,
            total_settled: 0,
            total_fees_collected: 0,
            settlement_count: 0,
            created_at: timestamp::now_microseconds(),
            last_settlement_at: 0,
            is_active: true,
        };

        move_to(owner, vault);

        // Emit vault initialization event
        events::emit_vault_initialized(
            owner_addr,
            owner_addr,
            string::utf8(b"usdc_vault_initialized")
        );
    }

    /// Deposit USDC into the vault
    public entry fun deposit_usdc(
        depositor: &signer,
        vault_owner: address,
        amount: u64
    ) acquires Vault {
        let depositor_addr = signer::address_of(depositor);
        
        // Validate inputs
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(vault_owner);
        
        // Withdraw USDC from depositor and add to vault
        let deposit_coin = coin::withdraw<USDC>(depositor, amount);
        coin::merge(&mut vault.usdc_balance, deposit_coin);
        
        // Update vault statistics
        vault.total_deposited = vault.total_deposited + amount;
        
        let new_balance = coin::value(&vault.usdc_balance);
        
        // Emit deposit event
        events::emit_vault_deposit(
            vault_owner,
            vault_owner,
            depositor_addr,
            amount,
            new_balance
        );
    }

    /// Process a USDC settlement (internal function)
    public fun process_settlement(
        vault_owner: address,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        fee: u64,
    ): bool acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(vault_owner);
        
        // Check if settlement already processed
        if (table::contains(&vault.processed_settlements, source_tx_hash)) {
            return false
        };
        
        // Check sufficient balance
        let total_needed = amount + fee;
        let current_balance = coin::value(&vault.usdc_balance);
        if (current_balance < total_needed) {
            return false
        };
        
        // Mark settlement as processed
        table::add(&mut vault.processed_settlements, source_tx_hash, true);
        
        // Extract USDC for settlement
        let settlement_coin = coin::extract(&mut vault.usdc_balance, amount);
        let fee_coin = coin::extract(&mut vault.usdc_balance, fee);
        
        // Transfer settlement amount to receiver
        coin::deposit(receiver, settlement_coin);
        
        // Keep fee in vault (or transfer to fee collector)
        coin::merge(&mut vault.usdc_balance, fee_coin);
        
        // Update statistics
        vault.total_settled = vault.total_settled + amount;
        vault.total_fees_collected = vault.total_fees_collected + fee;
        vault.settlement_count = vault.settlement_count + 1;
        vault.last_settlement_at = timestamp::now_microseconds();
        
        true
    }

    /// Withdraw USDC from vault (owner only)
    public entry fun withdraw_usdc(
        owner: &signer,
        amount: u64
    ) acquires Vault {
        let owner_addr = signer::address_of(owner);
        
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        
        // Verify ownership
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));
        
        // Check sufficient balance
        let current_balance = coin::value(&vault.usdc_balance);
        assert!(current_balance >= amount, error::invalid_state(E_INSUFFICIENT_BALANCE));
        
        // Extract and transfer USDC
        let withdraw_coin = coin::extract(&mut vault.usdc_balance, amount);
        coin::deposit(owner_addr, withdraw_coin);
        
        let remaining_balance = coin::value(&vault.usdc_balance);
        
        // Emit withdraw event
        events::emit_vault_withdraw(
            owner_addr,
            owner_addr,
            owner_addr,
            amount,
            remaining_balance,
            string::utf8(b"normal_withdrawal")
        );
    }

    /// Emergency withdraw all USDC (owner only)
    public entry fun emergency_withdraw_all(owner: &signer) acquires Vault {
        let owner_addr = signer::address_of(owner);
        
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        
        // Verify ownership
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));
        
        let total_amount = coin::value(&vault.usdc_balance);
        if (total_amount > 0) {
            let withdraw_coin = coin::extract_all(&mut vault.usdc_balance);
            coin::deposit(owner_addr, withdraw_coin);
            
            // Emit emergency withdraw event
            events::emit_vault_withdraw(
                owner_addr,
                owner_addr,
                owner_addr,
                total_amount,
                0,
                string::utf8(b"emergency_withdrawal")
            );
        };
    }

    // === View Functions ===

    /// Check if settlement has been processed
    public fun is_settlement_processed(vault_owner: address, source_tx_hash: String): bool acquires Vault {
        if (!exists<Vault>(vault_owner)) {
            return false
        };
        
        let vault = borrow_global<Vault>(vault_owner);
        table::contains(&vault.processed_settlements, source_tx_hash)
    }

    /// Get vault USDC balance
    public fun get_vault_balance(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        coin::value(&vault.usdc_balance)
    }

    /// Get total USDC settled
    public fun get_total_settled(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        vault.total_settled
    }

    /// Get total fees collected
    public fun get_total_fees_collected(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        vault.total_fees_collected
    }

    /// Get settlement count
    public fun get_settlement_count(vault_owner: address): u64 acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        vault.settlement_count
    }

    /// Get vault metadata
    public fun get_vault_metadata(vault_owner: address): VaultMetadata acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        
        VaultMetadata {
            owner: vault.owner,
            total_balance: coin::value(&vault.usdc_balance),
            total_deposited: vault.total_deposited,
            total_settled: vault.total_settled,
            settlement_count: vault.settlement_count,
            created_at: vault.created_at,
            last_activity: vault.last_settlement_at,
            is_active: vault.is_active,
        }
    }

    /// Check if vault exists
    public fun vault_exists(vault_owner: address): bool {
        exists<Vault>(vault_owner)
    }

    /// Get comprehensive vault info
    public fun get_vault_info(vault_owner: address): (u64, u64, u64, u64, u64, bool) acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        (
            coin::value(&vault.usdc_balance),    // current balance
            vault.total_settled,                 // total settled
            vault.total_fees_collected,          // total fees
            vault.settlement_count,              // settlement count
            vault.created_at,                    // created timestamp
            vault.is_active,                     // active status
        )
    }

    // === Demo Utilities ===

    /// Initialize demo vault with some USDC (for testing)
    public entry fun initialize_demo_vault(owner: &signer) {
        initialize_vault(owner);
        
        // Note: In a real demo, you'd need to have USDC in your account first
        // This function just initializes the vault structure
    }

    /// Simulate demo deposit (requires actual USDC)
    public entry fun demo_deposit(owner: &signer, amount: u64) acquires Vault {
        let owner_addr = signer::address_of(owner);
        deposit_usdc(owner, owner_addr, amount);
    }

    /// Get demo-friendly balance in USDC (with decimals)
    public fun get_balance_in_usdc(vault_owner: address): (u64, u64) acquires Vault {
        let micro_usdc = get_vault_balance(vault_owner);
        let usdc_whole = micro_usdc / 1000000;
        let usdc_decimals = micro_usdc % 1000000;
        (usdc_whole, usdc_decimals)
    }

    // === Internal Helpers ===

    /// Update vault statistics (internal)
    fun update_vault_stats(vault: &mut Vault) {
        // Emit status update event
        events::emit_vault_status_update(
            vault.owner,
            vault.owner,
            coin::value(&vault.usdc_balance),
            vault.total_settled,
            1, // We'll get actual relayer count from relayer module
            vault.last_settlement_at,
        );
    }

    // === Testing Functions ===

    #[test_only]
    public fun setup_test_vault(account: &signer) {
        initialize_vault(account);
    }

    #[test_only]
    public fun get_test_balance(vault_owner: address): u64 acquires Vault {
        get_vault_balance(vault_owner)
    }

    #[test_only]
    public fun test_process_settlement(
        vault_owner: address,
        tx_hash: String,
        receiver: address,
        amount: u64,
    ): bool acquires Vault {
        process_settlement(vault_owner, tx_hash, receiver, amount, 0)
    }
}