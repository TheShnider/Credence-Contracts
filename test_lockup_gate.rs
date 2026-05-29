#![cfg(test)]

use super::*;
use credence_errors::ContractError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};
use soroban_sdk::testutils::Ledger;
use soroban_sdk::testutils::LedgerInfo;

fn setup() -> (Env, Address, CredenceBondClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredenceBond, ());
    let client = CredenceBondClient::new(&env, &contract_id);
    client.initialize(&admin);
    (env, admin, client)
}

fn advance_time(e: &Env, secs: u64) {
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + secs,
        protocol_version: 22,
        sequence_number: e.ledger().sequence() + 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 16,
        max_entry_ttl: 31_536_000,
    });
}

/// Test that withdraw() panics when called before lock-up expiry
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_withdraw_before_lockup_panics() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &500_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Try to withdraw immediately (before lock-up expiry) - should panic
    client.withdraw(&100_i128);
}

/// Test that withdraw() panics halfway through lock-up
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_withdraw_halfway_through_lockup_panics() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &500_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance halfway through lock-up
    advance_time(&env, duration / 2);

    // Try to withdraw - should panic
    client.withdraw(&100_i128);
}

/// Test that withdraw() panics one second before expiry
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_withdraw_one_second_before_expiry_panics() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &500_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance to one second before expiry
    advance_time(&env, duration - 1);

    // Try to withdraw - should panic
    client.withdraw(&100_i128);
}

/// Test that withdraw() succeeds exactly at lock-up expiry
#[test]
fn test_withdraw_at_lockup_expiry_succeeds() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance to exactly lock-up end
    advance_time(&env, duration);

    // Withdraw should succeed
    let bond = client.withdraw(&100_i128);
    assert_eq!(bond.bonded_amount, amount - 100);
}

/// Test that withdraw() succeeds after lock-up expiry
#[test]
fn test_withdraw_after_lockup_succeeds() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance past lock-up end
    advance_time(&env, duration + 100);

    // Withdraw should succeed
    let bond = client.withdraw(&100_i128);
    assert_eq!(bond.bonded_amount, amount - 100);
}

/// Test that withdraw_early() applies penalty before lock-up expiry
#[test]
fn test_withdraw_early_before_lockup_applies_penalty() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config with 10% penalty
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &1000_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance halfway through lock-up
    advance_time(&env, duration / 2);

    // withdraw_early should succeed and reduce bonded amount
    let bond = client.withdraw_early(&100_i128);
    assert_eq!(bond.bonded_amount, amount - 100);
}

/// Test that withdraw_early() reverts when called after lock-up expiry
#[test]
fn test_withdraw_early_after_lockup_reverts() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &500_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance past lock-up end
    advance_time(&env, duration + 1);

    // withdraw_early should fail
    let err = client.try_withdraw_early(&100_i128).unwrap_err().unwrap();
    assert_eq!(err, ContractError::LockupNotExpired);
}

/// Test mutual exclusivity: withdraw and withdraw_early have non-overlapping valid time windows
#[test]
fn test_withdraw_and_withdraw_early_mutual_exclusivity() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &500_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Before lock-up: withdraw_early succeeds
    let bond = client.withdraw_early(&50_i128);
    assert_eq!(bond.bonded_amount, amount - 50);

    // Advance to exactly lock-up end
    advance_time(&env, duration);

    // At/after lock-up: withdraw succeeds
    let bond2 = client.withdraw(&50_i128);
    assert_eq!(bond2.bonded_amount, amount - 100);

    // withdraw_early fails after lock-up
    let err2 = client.try_withdraw_early(&50_i128).unwrap_err().unwrap();
    assert_eq!(err2, ContractError::LockupNotExpired);
}

/// Test that penalty cannot be bypassed via withdraw before lock-up
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_penalty_cannot_be_bypassed() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Set up early exit config with 10% penalty
    let treasury = Address::generate(&env);
    client.set_early_exit_config(&admin, &treasury, &1000_u32);

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Advance to 1 second before lock-up end
    advance_time(&env, duration - 1);

    // Attempt to bypass penalty by calling withdraw - should panic
    client.withdraw(&100_i128);
}

/// Test edge case: bond with zero duration
#[test]
fn test_zero_duration_bond_immediate_withdrawal() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 0_u64;

    // Create bond with zero duration
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Should be able to withdraw immediately (lock-up already expired)
    let bond = client.withdraw(&100_i128);
    assert_eq!(bond.bonded_amount, amount - 100);
}

/// Test edge case: fully slashed bond
#[test]
fn test_fully_slashed_bond_withdrawal() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Slash the entire bond
    client.slash(&admin, &amount);

    // Advance past lock-up
    advance_time(&env, duration + 1);

    // Try to withdraw - should fail with insufficient balance
    let err = client.try_withdraw(&100_i128).unwrap_err().unwrap();
    assert_eq!(err, ContractError::InsufficientBalance);
}

/// Test edge case: partial slash with withdrawal
#[test]
fn test_partial_slash_with_withdrawal() {
    let (env, admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;

    // Create bond
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Slash half the bond
    client.slash(&admin, &500_i128);

    // Advance past lock-up
    advance_time(&env, duration + 1);

    // Should be able to withdraw available balance (500)
    let bond = client.withdraw(&500_i128);
    assert_eq!(bond.bonded_amount, 500);
    assert_eq!(bond.slashed_amount, 500);

    // Try to withdraw more - should fail
    let err = client.try_withdraw(&1_i128).unwrap_err().unwrap();
    assert_eq!(err, ContractError::InsufficientBalance);
}

/// Test that lock-up check uses checked arithmetic to prevent overflow
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_lockup_check_overflow_protection() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    
    // Use maximum safe duration
    let duration = u64::MAX / 2;

    // Create bond - should succeed with overflow check
    client.create_bond(&owner, &amount, &duration, &false, &0_u64);

    // Try to withdraw immediately - should panic
    client.withdraw(&100_i128);
}

/// Test rolling bond with lock-up gate - before expiry
#[test]
#[should_panic(expected = "lock-up not expired")]
fn test_rolling_bond_lockup_gate_before_expiry() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;
    let notice_period = 100_u64;

    // Create rolling bond
    client.create_bond(&owner, &amount, &duration, &true, &notice_period);

    // Before lock-up: withdraw should panic
    client.withdraw(&100_i128);
}

/// Test rolling bond withdrawal after lock-up and notice period
#[test]
fn test_rolling_bond_lockup_gate_after_expiry() {
    let (env, _admin, client) = setup();
    let owner = Address::generate(&env);
    let amount = 1000_i128;
    let duration = 1000_u64;
    let notice_period = 100_u64;

    // Create rolling bond
    client.create_bond(&owner, &amount, &duration, &true, &notice_period);

    // Advance past lock-up
    advance_time(&env, duration + 1);

    // Request withdrawal
    client.request_withdrawal();

    // After notice period: withdraw should succeed
    advance_time(&env, notice_period + 1);
    let bond = client.withdraw(&100_i128);
    assert_eq!(bond.bonded_amount, amount - 100);
}
