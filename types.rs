#![no_std]

use soroban_sdk::{contracttype, Address, String};

pub const MAX_ATTESTATION_WEIGHT: u32 = 1_000_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub id: u64,
    pub verifier: Address,
    pub identity: Address,
    pub timestamp: u64,
    pub weight: u32,
    pub attestation_data: String,
    pub revoked: bool,
}

impl Attestation {
    pub fn validate_weight(weight: u32) {
        if weight > MAX_ATTESTATION_WEIGHT {
            panic!("attestation weight exceeds maximum");
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationDedupKey {
    pub verifier: Address,
    pub identity: Address,
    pub attestation_data: String,
}
