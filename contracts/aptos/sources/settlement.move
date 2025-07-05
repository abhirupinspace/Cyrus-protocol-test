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

    // Circle USDC: 0x69091fbab5f7d635ee7ac5098cf0c1efbe31d68fec0f2cd565e8d168daf52832::usdc::USDC
    struct USDC has copy, drop, store {}

    const E_NOT_OWNER: u64 = 1;
    const E_INSUFFICIENT_BALANCE: u64 = 2;
    const E_INVALID_INSTRUCTION: u64 = 3;
    const E_ALREADY_SETTLED: u64 = 4;
    const E_VAULT_NOT_INITIALIZED: u64 = 5;
    const E_UNAUTHORIZED_RELAYER: u64 = 6;
    const E_INVALID_AMOUNT: u64 = 7;
    const E_VAULT_ALREADY_EXISTS: u64 = 8;

    struct SettlementInstruction has copy, drop, store {
        source_chain: String,           // "solana"
        source_tx_hash: String,         // Solana transaction hash
        receiver: address,              // Aptos recipient address
        asset: String,                  // "USDC"
        amount: u64,                    // Amount in smallest unit (6 decimals for USDC)
        nonce: u64,                     // Unique nonce for this instruction
        timestamp: u64,                 // Timestamp from source chain
    }

    struct SettlementEvent has drop, store {
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        timestamp: u64,
        settlement_time: u64,
        nonce: u64,
    }


    struct DepositEvent has drop, store {
        depositor: address,
        amount: u64,
        new_vault_balance: u64,
        timestamp: u64,
    }

    // Vault resource to hold USDC and manage settlements
    struct Vault has key {
        usdc_balance: Coin<USDC>,
        owner: address,
        authorized_relayers: vector<address>,
        processed_instructions: Table<String, bool>, // tx_hash -> settled
        total_settled: u64,
        settlement_events: EventHandle<SettlementEvent>,
        deposit_events: EventHandle<DepositEvent>,
        vault_created_at: u64,
    }

    
    public entry fun initialize_vault(owner: &signer) {
        let owner_addr = signer::address_of(owner);
    
        assert!(!exists<Vault>(owner_addr), error::already_exists(E_VAULT_ALREADY_EXISTS));
        
        // Create vault with empty USDC balance
        let vault = Vault {
            usdc_balance: coin::zero<USDC>(),
            owner: owner_addr,
            authorized_relayers: vector::empty<address>(),
            processed_instructions: table::new<String, bool>(),
            total_settled: 0,
            settlement_events: account::new_event_handle<SettlementEvent>(owner),
            deposit_events: account::new_event_handle<DepositEvent>(owner),
            vault_created_at: timestamp::now_microseconds(),
        };

        move_to(owner, vault);
    }

    public entry fun deposit_usdc(owner: &signer, amount: u64) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        // Withdraw USDC from owner's account and add to vault
        let deposit_coin = coin::withdraw<USDC>(owner, amount);
        coin::merge(&mut vault.usdc_balance, deposit_coin);


        // Emit deposit event
        let deposit_event = DepositEvent {
            depositor: owner_addr,
            amount,
            new_vault_balance: coin::value(&vault.usdc_balance),
            timestamp: timestamp::now_microseconds(),
        };
        event::emit_event(&mut vault.deposit_events, deposit_event);
    }

    public entry fun add_relayer(owner: &signer, relayer: address) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        if (!vector::contains(&vault.authorized_relayers, &relayer)) {
            vector::push_back(&mut vault.authorized_relayers, relayer);
        };
    }

    public entry fun remove_relayer(owner: &signer, relayer: address) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        let (found, index) = vector::index_of(&vault.authorized_relayers, &relayer);
        if (found) {
            vector::remove(&mut vault.authorized_relayers, index);
        };
    }

    public entry fun settle(
        relayer: &signer,
        vault_owner: address,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        nonce: u64,
        source_timestamp: u64
    ) acquires Vault {
        let relayer_addr = signer::address_of(relayer);
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(vault_owner);

        let is_authorized = vault.owner == relayer_addr || 
                           vector::contains(&vault.authorized_relayers, &relayer_addr);
        assert!(is_authorized, error::permission_denied(E_UNAUTHORIZED_RELAYER));

        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(coin::value(&vault.usdc_balance) >= amount, 
                error::invalid_state(E_INSUFFICIENT_BALANCE));

        assert!(!table::contains(&vault.processed_instructions, source_tx_hash), 
                error::already_exists(E_ALREADY_SETTLED));

        table::add(&mut vault.processed_instructions, source_tx_hash, true);

        
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
            nonce,
        };
        event::emit_event(&mut vault.settlement_events, settlement_event);
    }

    // Emergency functions
    public entry fun emergency_withdraw(owner: &signer, amount: u64) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(coin::value(&vault.usdc_balance) >= amount, 
                error::invalid_state(E_INSUFFICIENT_BALANCE));

        let withdraw_coin = coin::extract(&mut vault.usdc_balance, amount);
        coin::deposit(owner_addr, withdraw_coin);
    }

    public entry fun emergency_withdraw_all(owner: &signer) acquires Vault {
        let owner_addr = signer::address_of(owner);
        assert!(exists<Vault>(owner_addr), error::not_found(E_VAULT_NOT_INITIALIZED));
        
        let vault = borrow_global_mut<Vault>(owner_addr);
        assert!(vault.owner == owner_addr, error::permission_denied(E_NOT_OWNER));

        let total_amount = coin::value(&vault.usdc_balance);
        if (total_amount > 0) {
            let withdraw_coin = coin::extract_all(&mut vault.usdc_balance);
            coin::deposit(owner_addr, withdraw_coin);
        };
    }

    // View functions for monitoring and queries
    public fun is_settled(vault_owner: address, source_tx_hash: String): bool acquires Vault {
        if (!exists<Vault>(vault_owner)) {
            return false
        };
        
        let vault = borrow_global<Vault>(vault_owner);
        table::contains(&vault.processed_instructions, source_tx_hash)
    }

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

    public fun get_vault_info(vault_owner: address): (u64, u64, u64, vector<address>) acquires Vault {
        assert!(exists<Vault>(vault_owner), error::not_found(E_VAULT_NOT_INITIALIZED));
        let vault = borrow_global<Vault>(vault_owner);
        (
            coin::value(&vault.usdc_balance),
            vault.total_settled,
            vault.vault_created_at,
            vault.authorized_relayers
        )
    }

    public fun vault_exists(vault_owner: address): bool {
        exists<Vault>(vault_owner)
    }

    #[test_only]
    use std::string;

    #[test_only]
    public fun setup_test_account(account: &signer) {
        let addr = signer::address_of(account);
        if (!account::exists_at(addr)) {
            account::create_account_for_test(addr);
        };
    }

    #[test(admin = @cyrus_protocol)]
    public fun test_vault_initialization(admin: &signer) {
        setup_test_account(admin);
        initialize_vault(admin);
        
        let admin_addr = signer::address_of(admin);
        assert!(vault_exists(admin_addr), 1);
        assert!(get_vault_balance(admin_addr) == 0, 2);
        assert!(get_total_settled(admin_addr) == 0, 3);
    }

    #[test(admin = @cyrus_protocol, relayer = @0x123)]
    public fun test_relayer_management(admin: &signer, relayer: &signer) {
        setup_test_account(admin);
        setup_test_account(relayer);
        
        let admin_addr = signer::address_of(admin);
        let relayer_addr = signer::address_of(relayer);
        
        initialize_vault(admin);
        
        // Initially not authorized
        assert!(!is_authorized_relayer(admin_addr, relayer_addr), 1);
        
        // Add relayer
        add_relayer(admin, relayer_addr);
        assert!(is_authorized_relayer(admin_addr, relayer_addr), 2);
        
        // Remove relayer
        remove_relayer(admin, relayer_addr);
        assert!(!is_authorized_relayer(admin_addr, relayer_addr), 3);
    }

    #[test(admin = @cyrus_protocol)]
    public fun test_replay_protection(admin: &signer) {
        setup_test_account(admin);
        initialize_vault(admin);
        
        let admin_addr = signer::address_of(admin);
        let test_tx = string::utf8(b"test_tx_123");
        
        assert!(!is_settled(admin_addr, test_tx), 1);
    }
}