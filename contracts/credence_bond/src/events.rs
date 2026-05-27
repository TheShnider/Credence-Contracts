use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub fn emit_bond_created_v2(e: &Env, identity: &Address, amount: i128, duration: u64, is_rolling: bool, timestamp: u64) {
    e.events().publish(
        (symbol_short!("bond_c2"), identity),
        (amount, duration, is_rolling, timestamp)
    );
}

pub fn emit_bond_increased_v2(e: &Env, identity: &Address, added_amount: i128, total_balance: i128, timestamp: u64) {
    e.events().publish(
        (symbol_short!("bond_i2"), identity),
        (added_amount, total_balance, timestamp)
    );
}

pub fn emit_duration_extended_v2(e: &Env, identity: &Address, new_duration: u64, timestamp: u64) {
    e.events().publish(
        (symbol_short!("bond_e2"), identity),
        (new_duration, timestamp)
    );
}