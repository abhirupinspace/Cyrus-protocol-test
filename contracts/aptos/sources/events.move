module cyrus_protocol::events {
    use std::string::String;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::account;
    use aptos_framework::timestamp;

    // === Event Structures ===

    /// Emitted when a settlement is successfully processed
    struct SettlementProcessed has drop, store {
        source_chain: String,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        fee_collected: u64,
        relayer: address,
        vault_owner: address,
        nonce: u64,
        settlement_time: u64,
        gas_used: u64,
    }

    /// Emitted when a settlement fails
    struct SettlementFailed has drop, store {
        source_chain: String,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        relayer: address,
        vault_owner: address,
        reason: String,
        failure_time: u64,
        nonce: u64,
    }

    /// Emitted when USDC is deposited into a vault
    struct VaultDeposit has drop, store {
        vault_owner: address,
        depositor: address,
        amount: u64,
        new_vault_balance: u64,
        deposit_time: u64,
    }

    /// Emitted when USDC is withdrawn from a vault
    struct VaultWithdraw has drop, store {
        vault_owner: address,
        withdrawer: address,
        amount: u64,
        remaining_balance: u64,
        withdraw_time: u64,
        reason: String, // "emergency", "normal", etc.
    }

    /// Emitted when a relayer is authorized
    struct RelayerAuthorized has drop, store {
        vault_owner: address,
        relayer: address,
        authorized_by: address,
        authorization_time: u64,
    }

    /// Emitted when a relayer is deauthorized
    struct RelayerDeauthorized has drop, store {
        vault_owner: address,
        relayer: address,
        deauthorized_by: address,
        deauthorization_time: u64,
        reason: String,
    }

    /// Emitted when a vault is initialized
    struct VaultInitialized has drop, store {
        vault_owner: address,
        initialization_time: u64,
        initial_config: String,
    }

    /// Emitted when protocol configuration is updated
    struct ConfigurationUpdated has drop, store {
        admin: address,
        config_type: String, // "protocol_fee", "relayer_fee", "limits", etc.
        old_value: String,
        new_value: String,
        update_time: u64,
        version: u64,
    }

    /// Emitted for vault status updates
    struct VaultStatusUpdate has drop, store {
        vault_owner: address,
        total_balance: u64,
        total_settled: u64,
        active_relayers: u64,
        last_settlement_time: u64,
        update_time: u64,
    }

    /// Emitted for relayer performance updates
    struct RelayerPerformanceUpdate has drop, store {
        relayer: address,
        vault_owner: address,
        settlements_processed: u64,
        total_fees_earned: u64,
        success_rate: u64, // Basis points (10000 = 100%)
        last_activity: u64,
        update_time: u64,
    }

    // === Event Handles Container ===

    /// Container for all event handles
    struct EventContainer has key {
        settlement_processed: EventHandle<SettlementProcessed>,
        settlement_failed: EventHandle<SettlementFailed>,
        vault_deposit: EventHandle<VaultDeposit>,
        vault_withdraw: EventHandle<VaultWithdraw>,
        relayer_authorized: EventHandle<RelayerAuthorized>,
        relayer_deauthorized: EventHandle<RelayerDeauthorized>,
        vault_initialized: EventHandle<VaultInitialized>,
        configuration_updated: EventHandle<ConfigurationUpdated>,
        vault_status_update: EventHandle<VaultStatusUpdate>,
        relayer_performance_update: EventHandle<RelayerPerformanceUpdate>,
    }

    // === Initialization ===

    /// Initialize event container for an account
    public fun initialize_events(account: &signer) {
        if (!exists<EventContainer>(signer::address_of(account))) {
            let event_container = EventContainer {
                settlement_processed: account::new_event_handle<SettlementProcessed>(account),
                settlement_failed: account::new_event_handle<SettlementFailed>(account),
                vault_deposit: account::new_event_handle<VaultDeposit>(account),
                vault_withdraw: account::new_event_handle<VaultWithdraw>(account),
                relayer_authorized: account::new_event_handle<RelayerAuthorized>(account),
                relayer_deauthorized: account::new_event_handle<RelayerDeauthorized>(account),
                vault_initialized: account::new_event_handle<VaultInitialized>(account),
                configuration_updated: account::new_event_handle<ConfigurationUpdated>(account),
                vault_status_update: account::new_event_handle<VaultStatusUpdate>(account),
                relayer_performance_update: account::new_event_handle<RelayerPerformanceUpdate>(account),
            };
            move_to(account, event_container);
        }
    }

    // === Event Emission Functions ===

    /// Emit settlement processed event
    public fun emit_settlement_processed(
        account_addr: address,
        source_chain: String,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        fee_collected: u64,
        relayer: address,
        vault_owner: address,
        nonce: u64,
        gas_used: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let settlement_event = SettlementProcessed {
            source_chain,
            source_tx_hash,
            receiver,
            amount,
            fee_collected,
            relayer,
            vault_owner,
            nonce,
            settlement_time: timestamp::now_microseconds(),
            gas_used,
        };
        event::emit_event(&mut event_container.settlement_processed, settlement_event);
    }

    /// Emit settlement failed event
    public fun emit_settlement_failed(
        account_addr: address,
        source_chain: String,
        source_tx_hash: String,
        receiver: address,
        amount: u64,
        relayer: address,
        vault_owner: address,
        reason: String,
        nonce: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let failure_event = SettlementFailed {
            source_chain,
            source_tx_hash,
            receiver,
            amount,
            relayer,
            vault_owner,
            reason,
            failure_time: timestamp::now_microseconds(),
            nonce,
        };
        event::emit_event(&mut event_container.settlement_failed, failure_event);
    }

    /// Emit vault deposit event
    public fun emit_vault_deposit(
        account_addr: address,
        vault_owner: address,
        depositor: address,
        amount: u64,
        new_vault_balance: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let deposit_event = VaultDeposit {
            vault_owner,
            depositor,
            amount,
            new_vault_balance,
            deposit_time: timestamp::now_microseconds(),
        };
        event::emit_event(&mut event_container.vault_deposit, deposit_event);
    }

    /// Emit vault withdraw event
    public fun emit_vault_withdraw(
        account_addr: address,
        vault_owner: address,
        withdrawer: address,
        amount: u64,
        remaining_balance: u64,
        reason: String,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let withdraw_event = VaultWithdraw {
            vault_owner,
            withdrawer,
            amount,
            remaining_balance,
            withdraw_time: timestamp::now_microseconds(),
            reason,
        };
        event::emit_event(&mut event_container.vault_withdraw, withdraw_event);
    }

    /// Emit relayer authorized event
    public fun emit_relayer_authorized(
        account_addr: address,
        vault_owner: address,
        relayer: address,
        authorized_by: address,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let auth_event = RelayerAuthorized {
            vault_owner,
            relayer,
            authorized_by,
            authorization_time: timestamp::now_microseconds(),
        };
        event::emit_event(&mut event_container.relayer_authorized, auth_event);
    }

    /// Emit relayer deauthorized event
    public fun emit_relayer_deauthorized(
        account_addr: address,
        vault_owner: address,
        relayer: address,
        deauthorized_by: address,
        reason: String,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let deauth_event = RelayerDeauthorized {
            vault_owner,
            relayer,
            deauthorized_by,
            deauthorization_time: timestamp::now_microseconds(),
            reason,
        };
        event::emit_event(&mut event_container.relayer_deauthorized, deauth_event);
    }

    /// Emit vault initialized event
    public fun emit_vault_initialized(
        account_addr: address,
        vault_owner: address,
        initial_config: String,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let init_event = VaultInitialized {
            vault_owner,
            initialization_time: timestamp::now_microseconds(),
            initial_config,
        };
        event::emit_event(&mut event_container.vault_initialized, init_event);
    }

    /// Emit configuration updated event
    public fun emit_configuration_updated(
        account_addr: address,
        admin: address,
        config_type: String,
        old_value: String,
        new_value: String,
        version: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let config_event = ConfigurationUpdated {
            admin,
            config_type,
            old_value,
            new_value,
            update_time: timestamp::now_microseconds(),
            version,
        };
        event::emit_event(&mut event_container.configuration_updated, config_event);
    }

    /// Emit vault status update event
    public fun emit_vault_status_update(
        account_addr: address,
        vault_owner: address,
        total_balance: u64,
        total_settled: u64,
        active_relayers: u64,
        last_settlement_time: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let status_event = VaultStatusUpdate {
            vault_owner,
            total_balance,
            total_settled,
            active_relayers,
            last_settlement_time,
            update_time: timestamp::now_microseconds(),
        };
        event::emit_event(&mut event_container.vault_status_update, status_event);
    }

    /// Emit relayer performance update event
    public fun emit_relayer_performance_update(
        account_addr: address,
        relayer: address,
        vault_owner: address,
        settlements_processed: u64,
        total_fees_earned: u64,
        success_rate: u64,
        last_activity: u64,
    ) acquires EventContainer {
        let event_container = borrow_global_mut<EventContainer>(account_addr);
        let performance_event = RelayerPerformanceUpdate {
            relayer,
            vault_owner,
            settlements_processed,
            total_fees_earned,
            success_rate,
            last_activity,
            update_time: timestamp::now_microseconds(),
        };
        event::emit_event(&mut event_container.relayer_performance_update, performance_event);
    }

    // === View Functions ===

    /// Check if events are initialized for an account
    public fun events_initialized(account: address): bool {
        exists<EventContainer>(account)
    }

    // === Demo Utilities ===

    /// Emit a demo settlement for testing
    public fun emit_demo_settlement(
        account_addr: address,
        demo_amount: u64,
        demo_receiver: address,
    ) acquires EventContainer {
        emit_settlement_processed(
            account_addr,
            std::string::utf8(b"solana"),
            std::string::utf8(b"demo_tx_hash_12345"),
            demo_receiver,
            demo_amount,
            demo_amount / 1000, // 0.1% fee
            @0x1234, // Demo relayer address
            account_addr,
            1, // Demo nonce
            5000, // Demo gas used
        );
    }

    /// Emit demo events for dashboard testing
    public fun emit_demo_activity(account_addr: address) acquires EventContainer {
        // Emit vault deposit
        emit_vault_deposit(account_addr, account_addr, account_addr, 10000000000, 10000000000);
        
        // Emit relayer authorization
        emit_relayer_authorized(account_addr, account_addr, @0x1234, account_addr);
        
        // Emit some settlements
        emit_demo_settlement(account_addr, 1000000, @0xdemo1);
        emit_demo_settlement(account_addr, 2500000, @0xdemo2);
        emit_demo_settlement(account_addr, 500000, @0xdemo3);
        
        // Emit status update
        emit_vault_status_update(
            account_addr,
            account_addr,
            9996000000, // Balance after settlements
            4000000,     // Total settled
            1,           // Active relayers
            timestamp::now_microseconds()
        );
    }

    // === Testing Functions ===

    #[test_only]
    public fun initialize_test_events(account: &signer) {
        initialize_events(account);
    }
}