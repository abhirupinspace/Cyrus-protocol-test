module cyrus_protocol::relayer {
    use std::string::{Self, String};
    use std::signer;
    use std::error;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use cyrus_protocol::types::{Self, RelayerInfo};
    use cyrus_protocol::events;
    use cyrus_protocol::config;

    // === Error Codes ===
    const E_NOT_OWNER: u64 = 1;
    const E_RELAYER_NOT_FOUND: u64 = 2;
    const E_RELAYER_ALREADY_AUTHORIZED: u64 = 3;
    const E_UNAUTHORIZED_RELAYER: u64 = 4;
    const E_MAX_RELAYERS_REACHED: u64 = 5;
    const E_RELAYER_REGISTRY_NOT_INITIALIZED: u64 = 6;
    const E_INVALID_RELAYER_ADDRESS: u64 = 7;

    /// Registry for managing authorized relayers per vault
    struct RelayerRegistry has key {
        vault_owner: address,
        authorized_relayers: Table<address, RelayerInfo>,
        relayer_list: vector<address>, // For iteration
        max_relayers: u64,
        created_at: u64,
        updated_at: u64,
    }

    /// Global relayer performance metrics
    struct GlobalRelayerStats has key {
        total_relayers: u64,
        total_settlements_processed: u64,
        total_fees_distributed: u64,
        most_active_relayer: address,
        last_updated: u64,
    }

    // === Initialization ===

    /// Initialize relayer registry for a vault owner
    public entry fun initialize_relayer_registry(
        vault_owner: &signer,
        max_relayers: u64
    ) {
        let vault_owner_addr = signer::address_of(vault_owner);
        
        assert!(!exists<RelayerRegistry>(vault_owner_addr), 
                error::already_exists(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        // Initialize events if not already done
        events::initialize_events(vault_owner);
        
        let registry = RelayerRegistry {
            vault_owner: vault_owner_addr,
            authorized_relayers: table::new<address, RelayerInfo>(),
            relayer_list: vector::empty<address>(),
            max_relayers,
            created_at: timestamp::now_microseconds(),
            updated_at: timestamp::now_microseconds(),
        };
        
        move_to(vault_owner, registry);
    }

    /// Initialize with default settings
    public entry fun initialize_default_registry(vault_owner: &signer) {
        initialize_relayer_registry(vault_owner, 10); // Default max 10 relayers
    }

    // === Relayer Management ===

    /// Authorize a new relayer
    public entry fun authorize_relayer(
        vault_owner: &signer,
        relayer_address: address
    ) acquires RelayerRegistry {
        let vault_owner_addr = signer::address_of(vault_owner);
        
        assert!(exists<RelayerRegistry>(vault_owner_addr), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        assert!(relayer_address != @0x0, 
                error::invalid_argument(E_INVALID_RELAYER_ADDRESS));
        
        let registry = borrow_global_mut<RelayerRegistry>(vault_owner_addr);
        
        // Check if already authorized
        assert!(!table::contains(&registry.authorized_relayers, relayer_address),
                error::already_exists(E_RELAYER_ALREADY_AUTHORIZED));
        
        // Check max relayers limit
        assert!(vector::length(&registry.relayer_list) < registry.max_relayers,
                error::resource_exhausted(E_MAX_RELAYERS_REACHED));
        
        // Create relayer info
        let relayer_info = types::new_relayer_info(relayer_address);
        
        // Add to registry
        table::add(&mut registry.authorized_relayers, relayer_address, relayer_info);
        vector::push_back(&mut registry.relayer_list, relayer_address);
        registry.updated_at = timestamp::now_microseconds();
        
        // Emit authorization event
        events::emit_relayer_authorized(
            vault_owner_addr,
            vault_owner_addr,
            relayer_address,
            vault_owner_addr
        );
    }

    /// Deauthorize a relayer
    public entry fun deauthorize_relayer(
        vault_owner: &signer,
        relayer_address: address,
        reason: String
    ) acquires RelayerRegistry {
        let vault_owner_addr = signer::address_of(vault_owner);
        
        assert!(exists<RelayerRegistry>(vault_owner_addr), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        let registry = borrow_global_mut<RelayerRegistry>(vault_owner_addr);
        
        // Check if relayer exists
        assert!(table::contains(&registry.authorized_relayers, relayer_address),
                error::not_found(E_RELAYER_NOT_FOUND));
        
        // Remove from registry
        table::remove(&mut registry.authorized_relayers, relayer_address);
        
        // Remove from list
        let (found, index) = vector::index_of(&registry.relayer_list, &relayer_address);
        if (found) {
            vector::remove(&mut registry.relayer_list, index);
        };
        
        registry.updated_at = timestamp::now_microseconds();
        
        // Emit deauthorization event
        events::emit_relayer_deauthorized(
            vault_owner_addr,
            vault_owner_addr,
            relayer_address,
            vault_owner_addr,
            reason
        );
    }

    /// Update relayer performance metrics
    public fun update_relayer_performance(
        vault_owner: address,
        relayer_address: address,
        settlements_processed: u64,
        fees_earned: u64
    ) acquires RelayerRegistry {
        assert!(exists<RelayerRegistry>(vault_owner), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        let registry = borrow_global_mut<RelayerRegistry>(vault_owner);
        
        if (table::contains(&registry.authorized_relayers, relayer_address)) {
            let relayer_info = table::borrow_mut(&mut registry.authorized_relayers, relayer_address);
            types::update_relayer_activity(relayer_info, settlements_processed, fees_earned);
            
            // Emit performance update event
            events::emit_relayer_performance_update(
                vault_owner,
                relayer_address,
                vault_owner,
                types::get_relayer_settlements(relayer_info),
                types::get_relayer_fees(relayer_info),
                10000, // 100% success rate placeholder
                timestamp::now_microseconds()
            );
        };
    }

    // === Authorization Checks ===

    /// Check if a relayer is authorized for a vault
    public fun is_relayer_authorized(
        vault_owner: address,
        relayer_address: address
    ): bool acquires RelayerRegistry {
        if (!exists<RelayerRegistry>(vault_owner)) {
            return false
        };
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        
        // Owner is always authorized
        if (vault_owner == relayer_address) {
            return true
        };
        
        // Check if in authorized list and active
        if (table::contains(&registry.authorized_relayers, relayer_address)) {
            let relayer_info = table::borrow(&registry.authorized_relayers, relayer_address);
            return types::is_relayer_active(relayer_info)
        };
        
        false
    }

    /// Verify relayer authorization (throws error if not authorized)
    public fun verify_relayer_authorization(
        vault_owner: address,
        relayer_address: address
    ) acquires RelayerRegistry {
        assert!(is_relayer_authorized(vault_owner, relayer_address),
                error::permission_denied(E_UNAUTHORIZED_RELAYER));
    }

    // === View Functions ===

    /// Get all authorized relayers for a vault
    public fun get_authorized_relayers(vault_owner: address): vector<address> acquires RelayerRegistry {
        if (!exists<RelayerRegistry>(vault_owner)) {
            return vector::empty<address>()
        };
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        registry.relayer_list
    }

    /// Get relayer information
    public fun get_relayer_info(
        vault_owner: address,
        relayer_address: address
    ): RelayerInfo acquires RelayerRegistry {
        assert!(exists<RelayerRegistry>(vault_owner), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        assert!(table::contains(&registry.authorized_relayers, relayer_address),
                error::not_found(E_RELAYER_NOT_FOUND));
        
        *table::borrow(&registry.authorized_relayers, relayer_address)
    }

    /// Get relayer count
    public fun get_relayer_count(vault_owner: address): u64 acquires RelayerRegistry {
        if (!exists<RelayerRegistry>(vault_owner)) {
            return 0
        };
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        vector::length(&registry.relayer_list)
    }

    /// Get max relayers allowed
    public fun get_max_relayers(vault_owner: address): u64 acquires RelayerRegistry {
        assert!(exists<RelayerRegistry>(vault_owner), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        registry.max_relayers
    }

    /// Check if registry exists
    public fun registry_exists(vault_owner: address): bool {
        exists<RelayerRegistry>(vault_owner)
    }

    /// Get registry creation time
    public fun get_registry_created_at(vault_owner: address): u64 acquires RelayerRegistry {
        assert!(exists<RelayerRegistry>(vault_owner), 
                error::not_found(E_RELAYER_REGISTRY_NOT_INITIALIZED));
        
        let registry = borrow_global<RelayerRegistry>(vault_owner);
        registry.created_at
    }

    // === Batch Operations ===

    /// Authorize multiple relayers at once
    public entry fun authorize_multiple_relayers(
        vault_owner: &signer,
        relayer_addresses: vector<address>
    ) acquires RelayerRegistry {
        let i = 0;
        let len = vector::length(&relayer_addresses);
        
        while (i < len) {
            let relayer_addr = *vector::borrow(&relayer_addresses, i);
            authorize_relayer(vault_owner, relayer_addr);
            i = i + 1;
        };
    }

    /// Deauthorize multiple relayers at once
    public entry fun deauthorize_multiple_relayers(
        vault_owner: &signer,
        relayer_addresses: vector<address>,
        reason: String
    ) acquires RelayerRegistry {
        let i = 0;
        let len = vector::length(&relayer_addresses);
        
        while (i < len) {
            let relayer_addr = *vector::borrow(&relayer_addresses, i);
            deauthorize_relayer(vault_owner, relayer_addr, reason);
            i = i + 1;
        };
    }

    // === Demo Utilities ===

    /// Initialize demo relayer setup
    public entry fun setup_demo_relayers(
        vault_owner: &signer,
        demo_relayer: address
    ) acquires RelayerRegistry {
        initialize_default_registry(vault_owner);
        authorize_relayer(vault_owner, demo_relayer);
    }

    /// Create demo relayer activity
    public entry fun simulate_demo_activity(
        vault_owner: address,
        relayer_address: address
    ) acquires RelayerRegistry {
        update_relayer_performance(vault_owner, relayer_address, 5, 50000); // 5 settlements, 0.05 USDC fees
    }

    // === Testing Functions ===

    #[test_only]
    public fun setup_test_registry(vault_owner: &signer) {
        initialize_default_registry(vault_owner);
    }

    #[test_only]
    public fun authorize_test_relayer(vault_owner: &signer, relayer: address) acquires RelayerRegistry {
        authorize_relayer(vault_owner, relayer);
    }

    #[test_only]
    public fun test_relayer_authorized(vault_owner: address, relayer: address): bool acquires RelayerRegistry {
        is_relayer_authorized(vault_owner, relayer)
    }
}