# Withdrawal

This document describes how to withdraw USDC from identity bonds in the Credence bond contract.

## Overview

The contract supports three withdrawal flows:

1. **withdraw_bond(amount)** — Penalty-free withdrawal after lock-up (and cooldown for rolling bonds). Primary method.
2. **withdraw(amount)** — Alias for withdraw_bond. Same behavior.
3. **withdraw_early(amount)** — Early exit before lock-up; applies penalty proportional to remaining time.

**IMPORTANT:** `withdraw()` and `withdraw_early()` are mutually exclusive:
- **Before lock-up expiry:** Only `withdraw_early()` is valid. Calling `withdraw()` will panic with "lock-up not expired; use withdraw_early".
- **At or after lock-up expiry:** Only `withdraw()` is valid. Calling `withdraw_early()` will revert with `LockupNotExpired` error.

This ensures that early exits always pay the penalty and cannot bypass it by calling `withdraw()` before lock-up ends.

## Lock-Up Period

For non-rolling bonds, the lock-up period is:

- **End time:** `bond_start + bond_duration`
- **Withdraw allowed:** When current time ≥ end time
- **Before lock-up:** Must use `withdraw_early` (penalty applies)

The lock-up gate is enforced in `withdraw()` by computing:
```rust
let end = bond_start.checked_add(bond_duration).expect("overflow");
if now < end {
    panic!("lock-up not expired; use withdraw_early");
}
```

This prevents users from bypassing the early-exit penalty that funds the treasury.

## Cooldown (Rolling Bonds)

For rolling bonds, an additional cooldown applies:

1. Call **request_withdrawal()** to signal intent to exit.
2. Wait `notice_period_duration` seconds.
3. After the notice period elapses, call **withdraw_bond(amount)** or **withdraw(amount)**.

Withdrawal is only allowed when both:

- Lock-up has expired (for initial lock-up period)
- `withdrawal_requested_at + notice_period_duration ≤ now`

## USDC Transfer

- The contract holds USDC deposited via `create_bond` and `top_up`.
- On successful withdrawal, USDC is transferred from the contract to the identity owner.
- No penalty: full `amount` is sent to the identity.
- With `withdraw_early`, `amount - penalty` goes to the identity and `penalty` to the treasury.

## Partial Withdrawals

Partial withdrawals are supported. You may call `withdraw_bond` multiple times until the available balance is exhausted. Each call:

- Transfers the requested amount to the identity.
- Updates bond state (bonded_amount, tier).
- Reduces the available balance.

Available balance = `bonded_amount - slashed_amount`.

## Functions

### withdraw_bond(amount)

Primary withdrawal method. Enforces:

- Lock-up elapsed (non-rolling) or notice period elapsed (rolling).
- Amount ≤ available balance.
- Transfers USDC to identity owner.
- Updates bond state and tier.

### withdraw(amount)

Alias for `withdraw_bond`. Same behavior and validation. **Only valid after lock-up expiry.**

Panics:
- "lock-up not expired; use withdraw_early" if called before lock-up ends

### withdraw_early(amount)

Use when lock-up has not ended. Applies early-exit penalty; see [early-exit.md](early-exit.md). **Only valid before lock-up expiry.**

Errors:
- `LockupNotExpired` (204) if called after lock-up ends

## Requirements Before Withdrawal

- Token must be configured via `set_token`.
- Identity must have a bond with sufficient available balance.
- For non-rolling bonds: lock-up must have elapsed.
- For rolling bonds: withdrawal must be requested and notice period must have elapsed.

## Security

The lock-up gate prevents penalty bypass attacks:

1. User creates bond with 365-day lock-up
2. User attempts to withdraw on day 364 using `withdraw()` to avoid penalty
3. Contract panics with "lock-up not expired; use withdraw_early"
4. User must use `withdraw_early()` which applies the penalty

This ensures the treasury receives penalties from all early exits, maintaining protocol economics.
