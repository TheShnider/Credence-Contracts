//! Tests for cursor-paginated `get_pending_claims_page` (issue #654).
//!
//! Covers: empty set; single page exactly `limit`; `start_after` past the end;
//! `limit > MAX_PAGE_LIMIT` is clamped; and pages reassembling into the same set
//! as the unbounded read.

#![cfg(test)]

extern crate std;

use crate::{
    claims::{self, ClaimType, MAX_PAGE_LIMIT},
    CredenceBond,
};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Symbol, Vec};

fn setup(e: &Env) -> Address {
    e.mock_all_auths();
    e.register(CredenceBond, ())
}

/// Seed `n` claims for `user` inside the contract context and return their ids.
fn seed_claims(e: &Env, contract_id: &Address, user: &Address, n: u32) -> std::vec::Vec<u64> {
    e.as_contract(contract_id, || {
        let mut ids = std::vec::Vec::new();
        for i in 0..n {
            let id = claims::add_pending_claim(
                e,
                user,
                ClaimType::VerifierReward,
                1_000 + i as i128,
                i as u64,
                Some(Symbol::new(e, "seed")),
            );
            ids.push(id);
        }
        ids
    })
}

#[test]
fn empty_set_returns_empty_page_and_none() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);

    e.as_contract(&contract_id, || {
        let (page, cursor) = claims::get_pending_claims_page(&e, &user, 0, 10);
        assert_eq!(page.len(), 0);
        assert_eq!(cursor, None);
    });
}

#[test]
fn zero_limit_returns_empty_page() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);
    seed_claims(&e, &contract_id, &user, 5);

    e.as_contract(&contract_id, || {
        let (page, cursor) = claims::get_pending_claims_page(&e, &user, 0, 0);
        assert_eq!(page.len(), 0);
        assert_eq!(cursor, None);
    });
}

#[test]
fn single_page_exactly_limit_returns_cursor_then_exhausts() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);
    let ids = seed_claims(&e, &contract_id, &user, 3);

    e.as_contract(&contract_id, || {
        // Page of exactly the available count: returns the page plus a cursor at
        // the last id (standard cursor semantics — exhaustion confirmed next call).
        let (page, cursor) = claims::get_pending_claims_page(&e, &user, 0, 3);
        assert_eq!(page.len(), 3);
        assert_eq!(cursor, Some(*ids.last().unwrap()));

        // Resuming past the last id yields an empty, exhausted page.
        let (page2, cursor2) =
            claims::get_pending_claims_page(&e, &user, *ids.last().unwrap(), 3);
        assert_eq!(page2.len(), 0);
        assert_eq!(cursor2, None);
    });
}

#[test]
fn start_after_past_the_end_returns_empty() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);
    let ids = seed_claims(&e, &contract_id, &user, 4);
    let max_id = *ids.last().unwrap();

    e.as_contract(&contract_id, || {
        let (page, cursor) = claims::get_pending_claims_page(&e, &user, max_id + 100, 10);
        assert_eq!(page.len(), 0);
        assert_eq!(cursor, None);
    });
}

#[test]
fn limit_is_clamped_to_max_page_limit() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);
    // Seed more than MAX_PAGE_LIMIT so a clamp is observable.
    let n = MAX_PAGE_LIMIT + 10;
    seed_claims(&e, &contract_id, &user, n);

    e.as_contract(&contract_id, || {
        // Request a huge limit; the page must never exceed MAX_PAGE_LIMIT.
        let (page, cursor) = claims::get_pending_claims_page(&e, &user, 0, u32::MAX);
        assert_eq!(page.len(), MAX_PAGE_LIMIT);
        assert!(cursor.is_some(), "more claims remain, cursor must be Some");
    });
}

#[test]
fn pages_reassemble_into_full_set_in_order() {
    let e = Env::default();
    let contract_id = setup(&e);
    let user = Address::generate(&e);
    let n = 23u32;
    seed_claims(&e, &contract_id, &user, n);

    e.as_contract(&contract_id, || {
        let full = claims::get_pending_claims(&e, &user);

        // Walk pages of 7 until exhausted, concatenating.
        let mut reassembled: Vec<claims::PendingClaim> = Vec::new(&e);
        let mut cursor = 0u64;
        let mut last_seen = 0u64;
        loop {
            let (page, next) = claims::get_pending_claims_page(&e, &user, cursor, 7);
            for c in page.iter() {
                // Strictly increasing claim_id across all pages.
                assert!(c.claim_id > last_seen, "claim ids must be monotonic");
                last_seen = c.claim_id;
                reassembled.push_back(c);
            }
            match next {
                Some(nc) => cursor = nc,
                None => break,
            }
        }

        assert_eq!(reassembled.len(), full.len());
        assert_eq!(reassembled.len(), n);
        for i in 0..full.len() {
            assert_eq!(reassembled.get(i).unwrap().claim_id, full.get(i).unwrap().claim_id);
        }
    });
}
