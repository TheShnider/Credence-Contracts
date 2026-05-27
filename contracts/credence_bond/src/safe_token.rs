use soroban_sdk::{Address, Env, token};
use crate::storage;

pub fn transfer_in(e: &Env, from: &Address, amount: i128) {
    let token_addr = storage::get_token(e);
    let client = token::Client::new(e, &token_addr);
    let contract = e.current_contract_address();

    let balance_before = client.balance(&contract);
    client.transfer_from(&contract, from, &contract, &amount);
    let balance_after = client.balance(&contract);

    if (balance_after - balance_before) != amount {
        panic!("unsupported token: transfer amount mismatch (code 213)");
    }
}

pub fn transfer_out(e: &Env, to: &Address, amount: i128) {
    let token_addr = storage::get_token(e);
    let client = token::Client::new(e, &token_addr);
    
    // Standard transfer out (SDK client panics on failure)
    client.transfer(&e.current_contract_address(), to, &amount);
}