# Early Exit Penalty

Penalty charged when users withdraw before the lock-up period ends.
Penalty is configurable and attributed to the protocol treasury.

## Overview

The early exit penalty system ensures that users who exit their bond before the lock-up period ends pay a proportional penalty to the treasury. This maintains protocol economics and prevents users from bypassing lock-up commitments.

**CRITICAL SECURITY:** The `withdraw()` function enforces lock-up expiry and will panic with "lock-up not expired; use withdraw_early" if called before lock-up ends. Users attempting early exit MUST use `withdraw_early()`, which applies the penalty. This prevents penalty bypass attacks.

## Configuration

| Field | Description |
|-------|-------------|
| `treasury` | Address that receives penalty amounts. |
| `penalty_bps` | Rate in basis points. **Must be in `[0, 10 000]`** (0 % – 100 %). Values above 10 000 are rejected with `ContractError::InvalidPenaltyBps` (211). |

Set via `set_early_exit_config(admin, treasury, penalty_bps)`. Admin-only.

### Config-changed event

Every successful call to `set_early_exit_config` emits `"early_exit_cfg_set"` with:

```
(old_penalty_bps: u32, new_penalty_bps: u32, treasury: Address)
```

`old_penalty_bps` is `0` when no previous configuration existed.

## Penalty Formula

```
penalty = (amount × penalty_bps / 10_000) × (remaining_time / total_duration)
```

- **remaining_time**: Time left until lock-up end (`end - now`).
- **total_duration**: Bond duration at creation.

The penalty is proportional to the fraction of the lock period that remains.

### Clamping guarantee

The computed penalty is **always clamped to `[0, amount]`**.  
This means:
- The user's net withdrawal (`amount - penalty`) is always ≥ 0.
- An operator cannot accidentally configure a penalty that exceeds 100 % of the
  withdrawn amount, even if `calculate_penalty` is called directly with a large
  `penalty_bps` value.

## Validation rules

| Check | Error |
|-------|-------|
| `penalty_bps > 10_000` | `ContractError::InvalidPenaltyBps` (211) |
| Config not set when `withdraw_early` is called | `ContractError::EarlyExitConfigNotSet` (210) |

### Example

- Bond: 1000 USDC, 365 days duration
- Penalty rate: 10% (1000 bps)
- Withdraw 500 USDC after 182 days (halfway through)
- Remaining time: 183 days
- Penalty: (500 * 1000 / 10000) * (183 / 365) = 50 * 0.5 = 25 USDC
- User receives: 475 USDC
- Treasury receives: 25 USDC

## Functions

### `set_early_exit_config(admin, treasury, penalty_bps)`

Stores the early-exit configuration. Rejects `penalty_bps > 10_000`.
Emits `"early_exit_cfg_set"`.

**Valid time window:** Only before lock-up expiry (`now < bond_start + bond_duration`)

**Errors:**
- `LockupNotExpired` (204) - if called at or after lock-up expiry; use `withdraw()` instead

### withdraw(amount)

Withdraws `amount` before lock-up end. Computes and clamps the penalty,
then emits `"early_exit_penalty"` with `(identity, amount, penalty, treasury)`.
In a full implementation the token transfer sends `amount - penalty` to the user
and `penalty` to the treasury.

### `withdraw(amount)`

Use after lock-up or after the rolling-bond notice period. No penalty.

**Valid time window:** Only at or after lock-up expiry (`now >= bond_start + bond_duration`)

**Errors:**
- `LockupStillActive` (217) - if called before lock-up expiry; use `withdraw_early()` instead

## Mutual Exclusivity

The two withdrawal functions have non-overlapping valid time windows:

| Time | withdraw() | withdraw_early() |
|------|-----------|------------------|
| Before lock-up end | ❌ Panics: "lock-up not expired" | ✅ Succeeds with penalty |
| At lock-up end | ✅ Succeeds, no penalty | ❌ Reverts with `LockupNotExpired` |
| After lock-up end | ✅ Succeeds, no penalty | ❌ Reverts with `LockupNotExpired` |

This design ensures:
1. Early exits always pay the penalty
2. Post-lock-up withdrawals never pay a penalty
3. No way to bypass the penalty system

## Events

| Event | Payload |
|-------|---------|
| `"early_exit_cfg_set"` | `(old_penalty_bps, new_penalty_bps, treasury)` |
| `"early_exit_penalty"` | `(identity, withdraw_amount, penalty_amount, treasury)` |

## Security

- Penalty capped by amount and rate; no overflow in calculation.
- Config can only be set by admin.
- **Lock-up gate:** `withdraw()` enforces `now >= bond_start + bond_duration` before allowing withdrawal.
- **Early exit gate:** `withdraw_early()` enforces `now < bond_start + bond_duration` before applying penalty.
- Withdrawing after lock-up must use `withdraw`, not `withdraw_early`.
- Withdrawing before lock-up must use `withdraw_early`, not `withdraw`.

## Attack Prevention

### Penalty Bypass Attack (PREVENTED)

**Attack scenario:**
1. Attacker creates bond with 365-day lock-up
2. On day 364, attacker calls `withdraw()` to avoid penalty
3. Attacker receives full amount without paying penalty to treasury

**Prevention:**
The `withdraw()` function computes `end = bond_start + bond_duration` and requires `now >= end`. If called before lock-up expiry, it panics with "lock-up not expired; use withdraw_early", forcing the attacker to use `withdraw_early()` which applies the penalty.

```rust
// In withdraw():
let end = bond.bond_start.checked_add(bond.bond_duration).expect("overflow");
if now < end {
    panic!("lock-up not expired; use withdraw_early");
}
```

This ensures the treasury receives penalties from all early exits, maintaining protocol economics.
