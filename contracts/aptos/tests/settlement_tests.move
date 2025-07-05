#[test_only]
module cyrus_protocol::settlement_integration_tests {
    use std::string;
    use std::signer;
    use std::vector;
    use aptos_framework::account;
    use cyrus_protocol::settlement;

    #[test(admin = @cyrus_protocol)]
    public fun test_complete_vault_lifecycle(admin: &signer) {
        // Setup
        settlement::setup_test_account(admin);
        let admin_addr = signer::address_of(admin);
        
        // Test vault initialization
        settlement::initialize_vault(admin);
        assert!(settlement::vault_exists(admin_addr), 1);
        
        // Test initial state
        assert!(settlement::get_vault_balance(admin_addr) == 0, 2);
        assert!(settlement::get_total_settled(admin_addr) == 0, 3);
        
        // Test vault info
        let (balance, settled, created_at, relayers) = settlement::get_vault_info(admin_addr);
        assert!(balance == 0, 4);
        assert!(settled == 0, 5);
        assert!(created_at > 0, 6);
        assert!(vector::length(&relayers) == 0, 7);
    }

    #[test(admin = @cyrus_protocol, relayer1 = @0x123, relayer2 = @0x456)]
    public fun test_relayer_management_comprehensive(
        admin: &signer, 
        relayer1: &signer, 
        relayer2: &signer
    ) {
        // Setup accounts
        settlement::setup_test_account(admin);
        settlement::setup_test_account(relayer1);
        settlement::setup_test_account(relayer2);
        
        let admin_addr = signer::address_of(admin);
        let relayer1_addr = signer::address_of(relayer1);
        let relayer2_addr = signer::address_of(relayer2);
        
        // Initialize vault
        settlement::initialize_vault(admin);
        
        // Test owner is automatically authorized
        assert!(settlement::is_authorized_relayer(admin_addr, admin_addr), 1);
        
        // Test relayers are not initially authorized
        assert!(!settlement::is_authorized_relayer(admin_addr, relayer1_addr), 2);
        assert!(!settlement::is_authorized_relayer(admin_addr, relayer2_addr), 3);
        
        // Add first relayer
        settlement::add_relayer(admin, relayer1_addr);
        assert!(settlement::is_authorized_relayer(admin_addr, relayer1_addr), 4);
        assert!(!settlement::is_authorized_relayer(admin_addr, relayer2_addr), 5);
        
        // Add second relayer
        settlement::add_relayer(admin, relayer2_addr);
        assert!(settlement::is_authorized_relayer(admin_addr, relayer1_addr), 6);
        assert!(settlement::is_authorized_relayer(admin_addr, relayer2_addr), 7);
        
        // Test adding same relayer twice (should not duplicate)
        settlement::add_relayer(admin, relayer1_addr);
        let (_, _, _, relayers) = settlement::get_vault_info(admin_addr);
        assert!(vector::length(&relayers) == 2, 8); // Should still be 2, not 3
        
        // Remove first relayer
        settlement::remove_relayer(admin, relayer1_addr);
        assert!(!settlement::is_authorized_relayer(admin_addr, relayer1_addr), 9);
        assert!(settlement::is_authorized_relayer(admin_addr, relayer2_addr), 10);
        
        // Remove second relayer
        settlement::remove_relayer(admin, relayer2_addr);
        assert!(!settlement::is_authorized_relayer(admin_addr, relayer2_addr), 11);
        
        // Verify final state
        let (_, _, _, final_relayers) = settlement::get_vault_info(admin_addr);
        assert!(vector::length(&final_relayers) == 0, 12);
    }

    #[test(admin = @cyrus_protocol)]
    public fun test_replay_protection_comprehensive(admin: &signer) {
        settlement::setup_test_account(admin);
        settlement::initialize_vault(admin);
        
        let admin_addr = signer::address_of(admin);
        
        // Test different transaction hashes
        let tx_hash_1 = string::utf8(b"solana_tx_hash_001");
        let tx_hash_2 = string::utf8(b"solana_tx_hash_002");
        let tx_hash_3 = string::utf8(b"different_format_tx_123456");
        
        // Initially none should be settled
        assert!(!settlement::is_settled(admin_addr, tx_hash_1), 1);
        assert!(!settlement::is_settled(admin_addr, tx_hash_2), 2);
        assert!(!settlement::is_settled(admin_addr, tx_hash_3), 3);
        
        // Test with non-existent vault
        let fake_addr = @0x999;
        assert!(!settlement::is_settled(fake_addr, tx_hash_1), 4);
    }

    #[test(admin = @cyrus_protocol)]
    #[expected_failure(abort_code = 8, location = cyrus_protocol::settlement)]
    public fun test_double_vault_initialization(admin: &signer) {
        settlement::setup_test_account(admin);
        
        // First initialization should succeed
        settlement::initialize_vault(admin);
        
        // Second initialization should fail
        settlement::initialize_vault(admin);
    }

    #[test(admin = @cyrus_protocol, other = @0x999)]
    #[expected_failure(abort_code = 1, location = cyrus_protocol::settlement)]
    public fun test_unauthorized_relayer_addition(admin: &signer, other: &signer) {
        settlement::setup_test_account(admin);
        settlement::setup_test_account(other);
        
        let admin_addr = signer::address_of(admin);
        let other_addr = signer::address_of(other);
        
        settlement::initialize_vault(admin);
        
    
        settlement::add_relayer(other, other_addr);
    }

    #[test(admin = @cyrus_protocol)]
    #[expected_failure(abort_code = 5, location = cyrus_protocol::settlement)]
    public fun test_operations_without_vault(admin: &signer) {
        settlement::setup_test_account(admin);
        let admin_addr = signer::address_of(admin);
        
        // Try to get vault balance without initializing vault
        settlement::get_vault_balance(admin_addr);
    }

    #[test(admin = @cyrus_protocol)]
    public fun test_edge_cases(admin: &signer) {
        settlement::setup_test_account(admin);
        settlement::initialize_vault(admin);
        
        let admin_addr = signer::address_of(admin);
        
        // Test with empty string transaction hash
        let empty_hash = string::utf8(b"");
        assert!(!settlement::is_settled(admin_addr, empty_hash), 1);
        
        // Test with very long transaction hash
        let long_hash = string::utf8(b"very_long_transaction_hash_that_simulates_actual_blockchain_transaction_identifiers_which_can_be_quite_lengthy_in_practice");
        assert!(!settlement::is_settled(admin_addr, long_hash), 2);
        
        // Test vault info with fresh vault
        let (balance, settled, created_at, relayers) = settlement::get_vault_info(admin_addr);
        assert!(balance == 0, 3);
        assert!(settled == 0, 4);
        assert!(created_at > 0, 5);
        assert!(vector::length(&relayers) == 0, 6);
    }
}