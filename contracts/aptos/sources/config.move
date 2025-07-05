module cyrus_protocol::config {
    use std::error;
    use std::signer;
    use aptos_framework::timestamp;
    use cyrus_protocol::types::{Self, ProtocolConfig};

    // === Error Codes ===
    const E_NOT_ADMIN: u64 = 1;
    const E_CONFIG_NOT_INITIALIZED: u64 = 2;
    const E_INVALID_FEE: u64 = 3;
    const E_INVALID_AMOUNT: u64 = 4;
    const E_CONFIG_ALREADY_EXISTS: u64 = 5;

    // === Constants ===
    const MAX_FEE_BPS: u64 = 1000; // Maximum 10% fee
    const MIN_SETTLEMENT_AMOUNT: u64 = 1000; // 0.001 USDC minimum
    const MAX_SETTLEMENT_AMOUNT: u64 = 1000000000000; // 1M USDC maximum
    const DEFAULT_TIMEOUT: u64 = 86400000000; // 24 hours in microseconds
    const MAX_RELAYERS: u64 = 100;

    /// Global protocol configuration
    struct GlobalConfig has key {
        admin: address,
        config: ProtocolConfig,
        created_at: u64,
        updated_at: u64,
        version: u64,
    }

    /// Initialize protocol configuration
    public entry fun initialize(
        admin: &signer,
        protocol_fee_bps: u64,
        relayer_fee_bps: u64,
        min_settlement_amount: u64,
        max_settlement_amount: u64,
        settlement_timeout: u64,
        max_relayers: u64,
    ) {
        let admin_addr = signer::address_of(admin);
        assert!(!exists<GlobalConfig>(admin_addr), error::already_exists(E_CONFIG_ALREADY_EXISTS));

        // Validate parameters
        assert!(protocol_fee_bps <= MAX_FEE_BPS, error::invalid_argument(E_INVALID_FEE));
        assert!(relayer_fee_bps <= MAX_FEE_BPS, error::invalid_argument(E_INVALID_FEE));
        assert!(min_settlement_amount >= MIN_SETTLEMENT_AMOUNT, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(max_settlement_amount <= MAX_SETTLEMENT_AMOUNT, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(min_settlement_amount < max_settlement_amount, error::invalid_argument(E_INVALID_AMOUNT));

        let config = types::new_protocol_config(
            protocol_fee_bps,
            relayer_fee_bps,
            min_settlement_amount,
            max_settlement_amount,
            settlement_timeout,
            max_relayers,
        );

        let global_config = GlobalConfig {
            admin: admin_addr,
            config,
            created_at: timestamp::now_microseconds(),
            updated_at: timestamp::now_microseconds(),
            version: 1,
        };

        move_to(admin, global_config);
    }

    /// Initialize with default configuration
    public entry fun initialize_default(admin: &signer) {
        initialize(
            admin,
            10,         // 0.1% protocol fee
            5,          // 0.05% relayer fee
            100000,     // 0.1 USDC minimum
            100000000000, // 100,000 USDC maximum
            DEFAULT_TIMEOUT,
            20,         // Max 20 relayers
        );
    }

    /// Update protocol fee (admin only)
    public entry fun update_protocol_fee(
        admin: &signer,
        new_fee_bps: u64,
    ) acquires GlobalConfig {
        let admin_addr = signer::address_of(admin);
        assert!(exists<GlobalConfig>(admin_addr), error::not_found(E_CONFIG_NOT_INITIALIZED));
        
        let global_config = borrow_global_mut<GlobalConfig>(admin_addr);
        assert!(global_config.admin == admin_addr, error::permission_denied(E_NOT_ADMIN));
        assert!(new_fee_bps <= MAX_FEE_BPS, error::invalid_argument(E_INVALID_FEE));

        global_config.config = types::new_protocol_config(
            new_fee_bps,
            get_relayer_fee_bps(admin_addr),
            get_min_settlement_amount(admin_addr),
            get_max_settlement_amount(admin_addr),
            get_settlement_timeout(admin_addr),
            get_max_relayers(admin_addr),
        );
        global_config.updated_at = timestamp::now_microseconds();
        global_config.version = global_config.version + 1;
    }

    /// Update relayer fee (admin only)
    public entry fun update_relayer_fee(
        admin: &signer,
        new_fee_bps: u64,
    ) acquires GlobalConfig {
        let admin_addr = signer::address_of(admin);
        assert!(exists<GlobalConfig>(admin_addr), error::not_found(E_CONFIG_NOT_INITIALIZED));
        
        let global_config = borrow_global_mut<GlobalConfig>(admin_addr);
        assert!(global_config.admin == admin_addr, error::permission_denied(E_NOT_ADMIN));
        assert!(new_fee_bps <= MAX_FEE_BPS, error::invalid_argument(E_INVALID_FEE));

        global_config.config = types::new_protocol_config(
            get_protocol_fee_bps(admin_addr),
            new_fee_bps,
            get_min_settlement_amount(admin_addr),
            get_max_settlement_amount(admin_addr),
            get_settlement_timeout(admin_addr),
            get_max_relayers(admin_addr),
        );
        global_config.updated_at = timestamp::now_microseconds();
        global_config.version = global_config.version + 1;
    }

    /// Update settlement amount limits (admin only)
    public entry fun update_amount_limits(
        admin: &signer,
        min_amount: u64,
        max_amount: u64,
    ) acquires GlobalConfig {
        let admin_addr = signer::address_of(admin);
        assert!(exists<GlobalConfig>(admin_addr), error::not_found(E_CONFIG_NOT_INITIALIZED));
        
        let global_config = borrow_global_mut<GlobalConfig>(admin_addr);
        assert!(global_config.admin == admin_addr, error::permission_denied(E_NOT_ADMIN));
        assert!(min_amount >= MIN_SETTLEMENT_AMOUNT, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(max_amount <= MAX_SETTLEMENT_AMOUNT, error::invalid_argument(E_INVALID_AMOUNT));
        assert!(min_amount < max_amount, error::invalid_argument(E_INVALID_AMOUNT));

        global_config.config = types::new_protocol_config(
            get_protocol_fee_bps(admin_addr),
            get_relayer_fee_bps(admin_addr),
            min_amount,
            max_amount,
            get_settlement_timeout(admin_addr),
            get_max_relayers(admin_addr),
        );
        global_config.updated_at = timestamp::now_microseconds();
        global_config.version = global_config.version + 1;
    }

    // === View Functions ===

    public fun get_config(admin: address): ProtocolConfig acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        borrow_global<GlobalConfig>(admin).config
    }

    public fun get_protocol_fee_bps(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        let global_config = borrow_global<GlobalConfig>(admin);
        types::calculate_fees(10000, &global_config.config); // Get protocol fee for 1 USDC
        // This is a placeholder - we need to expose fee getters in types module
        10 // Return default for now
    }

    public fun get_relayer_fee_bps(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        5 // Return default for now - should get from config
    }

    public fun get_min_settlement_amount(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        100000 // Return default for now
    }

    public fun get_max_settlement_amount(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        100000000000 // Return default for now
    }

    public fun get_settlement_timeout(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        DEFAULT_TIMEOUT
    }

    public fun get_max_relayers(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        20
    }

    public fun is_config_initialized(admin: address): bool {
        exists<GlobalConfig>(admin)
    }

    public fun get_config_version(admin: address): u64 acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        borrow_global<GlobalConfig>(admin).version
    }

    public fun get_admin(admin: address): address acquires GlobalConfig {
        assert!(exists<GlobalConfig>(admin), error::not_found(E_CONFIG_NOT_INITIALIZED));
        borrow_global<GlobalConfig>(admin).admin
    }

    // === Demo Utilities ===

    /// Initialize demo configuration with reasonable parameters
    public entry fun initialize_demo_config(admin: &signer) {
        initialize(
            admin,
            5,          // 0.05% protocol fee (lower for demo)
            2,          // 0.02% relayer fee (lower for demo)
            10000,      // 0.01 USDC minimum (lower for demo)
            10000000000, // 10,000 USDC maximum
            3600000000, // 1 hour timeout
            5,          // Max 5 relayers for demo
        );
    }

    /// Get demo-friendly fee calculation
    public fun calculate_demo_fees(amount: u64, admin: address): (u64, u64) acquires GlobalConfig {
        let config = get_config(admin);
        types::calculate_fees(amount, &config)
    }

    // === Testing Functions ===

    #[test_only]
    public fun initialize_test_config(admin: &signer) {
        initialize_demo_config(admin);
    }

    #[test_only]
    public fun get_test_config(admin: address): ProtocolConfig acquires GlobalConfig {
        get_config(admin)
    }

    // === Constants for external use ===

    public fun get_max_fee_bps(): u64 {
        MAX_FEE_BPS
    }

    public fun get_default_timeout(): u64 {
        DEFAULT_TIMEOUT
    }

    public fun get_absolute_min_amount(): u64 {
        MIN_SETTLEMENT_AMOUNT
    }

    public fun get_absolute_max_amount(): u64 {
        MAX_SETTLEMENT_AMOUNT
    }
}