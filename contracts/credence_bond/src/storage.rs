use soroban_sdk::{contracttype, Address, Env};
use crate::Bond;
use credence_errors::ContractError;

#[contracttype]
pub enum DataKey {
    Bond(Address),
}

pub fn has_bond(e: &Env, identity: &Address) -> bool {
    e.storage().persistent().has(&DataKey::Bond(identity.clone()))
}

pub fn get_bond(e: &Env, identity: &Address) -> Result<Bond, ContractError> {
    e.storage()
        .persistent()
        .get(&DataKey::Bond(identity.clone()))
        .ok_or(ContractError::BondNotFound)
}

pub fn set_bond(e: &Env, identity: &Address, bond: &Bond) {
    e.storage().persistent().set(&DataKey::Bond(identity.clone()), bond);
    e.storage().persistent().extend_ttl(&DataKey::Bond(identity.clone()), 518400, 3110400); // ~30 days to 6 months
}