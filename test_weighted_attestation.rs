#![cfg(test)]

use crate::weighted_attestation;
use soroban_sdk::{Address, Env};

#[test]
fn test_weight_config_set_event_and_bounds() {
    let e = Env::default();
    weighted_attestation::set_weight_config(&e, 2_500, 1_000);
    let (multiplier, max_weight) = weighted_attestation::get_weight_config(&e);
    assert_eq!(multiplier, 2_500);
    assert_eq!(max_weight, 1_000);
}

#[test]
#[should_panic(expected = "multiplier_bps exceeds maximum")]
fn test_weight_config_multiplier_too_large() {
    let e = Env::default();
    weighted_attestation::set_weight_config(&e, weighted_attestation::MAX_WEIGHT_CONFIG_MULTIPLIER_BPS + 1, 100);
}

#[test]
#[should_panic(expected = "max_weight exceeds maximum")]
fn test_weight_config_max_weight_too_large() {
    let e = Env::default();
    weighted_attestation::set_weight_config(&e, 1_000, weighted_attestation::MAX_ATTESTATION_WEIGHT + 1);
}

#[test]
fn test_huge_stake_clamps_to_max_weight() {
    let e = Env::default();
    let attester = Address::random(&e);
    weighted_attestation::set_attester_stake(&e, &attester, i128::MAX);
    weighted_attestation::set_weight_config(&e, weighted_attestation::MAX_WEIGHT_CONFIG_MULTIPLIER_BPS, 1_000);
    let weight = weighted_attestation::compute_weight(&e, &attester);
    assert_eq!(weight, 1_000);
}

#[test]
fn test_zero_multiplier_yields_baseline_weight() {
    let e = Env::default();
    let attester = Address::random(&e);
    weighted_attestation::set_attester_stake(&e, &attester, 10);
    weighted_attestation::set_weight_config(&e, 0, 5);
    assert_eq!(weighted_attestation::compute_weight(&e, &attester), 1);
}

#[test]
fn test_max_weight_zero_returns_zero() {
    let e = Env::default();
    let attester = Address::random(&e);
    weighted_attestation::set_attester_stake(&e, &attester, 123);
    weighted_attestation::set_weight_config(&e, 10_000, 0);
    assert_eq!(weighted_attestation::compute_weight(&e, &attester), 0);
}

#[test]
fn test_compute_weight_does_not_overflow_for_max_stake() {
    let e = Env::default();
    let attester = Address::random(&e);
    weighted_attestation::set_attester_stake(&e, &attester, i128::MAX);
    weighted_attestation::set_weight_config(&e, weighted_attestation::MAX_WEIGHT_CONFIG_MULTIPLIER_BPS, weighted_attestation::MAX_ATTESTATION_WEIGHT);
    let weight = weighted_attestation::compute_weight(&e, &attester);
    assert!(weight <= weighted_attestation::MAX_ATTESTATION_WEIGHT);
}
