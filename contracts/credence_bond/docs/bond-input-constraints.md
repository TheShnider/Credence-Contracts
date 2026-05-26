# Bond Input Constraints

This document describes the input validation rules enforced by
`create_bond` in the `credence_bond` crate.

---

## Overview

`create_bond` is the single entry point for creating a bond. It validates all
parameters before constructing a `Bond` struct. Every invalid combination
returns a typed `ContractError` — no panics are used.

---

## Parameter Constraints

### `amount: i128`

| Rule | Error |
|------|-------|
| Must be strictly positive (`amount > 0`) | `ContractError::InvalidBondAmount` (214) |

The check is delegated to `is_valid_bond(amount)`, which is the single source
of truth for the amount rule. Passing `0` or any negative value returns
`InvalidBondAmount` immediately.

**Valid examples:** `1`, `1_000_000`, `i128::MAX`  
**Invalid examples:** `0`, `-1`, `i128::MIN`

---

### `bond_start: u64`

No direct constraint beyond the overflow guard below. Callers should pass the
current ledger timestamp.

---

### `duration: u64`

| Rule | Error |
|------|-------|
| Must be strictly positive (`duration > 0`) | `ContractError::InvalidBondDuration` (215) |

A zero-duration bond would expire instantly, making all time-based logic
(lock-up, early-exit, rolling withdrawal) nonsensical.

**Valid examples:** `1`, `3600`, `u64::MAX`  
**Invalid examples:** `0`

---

### `is_rolling: bool`

No constraint on the flag itself. When `true`, the notice-period rules below
apply.

---

### `notice_period_duration: u64`

These rules apply **only when `is_rolling` is `true`**. For non-rolling bonds
the field is stored as-is and not validated.

| Rule | Error |
|------|-------|
| Must be strictly positive (`notice_period_duration > 0`) | `ContractError::InvalidNoticePeriod` (216) |
| Must not exceed `duration` (`notice_period_duration <= duration`) | `ContractError::InvalidNoticePeriod` (216) |

A notice period of zero is undefined. A notice period longer than the bond
duration can never be satisfied, making rolling-bond withdrawal logic
nonsensical.

**Valid examples (given `duration = 3600`):** `1`, `1800`, `3600`  
**Invalid examples (given `duration = 3600`):** `0`, `3601`, `u64::MAX`

---

### Overflow guard

| Rule | Error |
|------|-------|
| `bond_start + duration` must not overflow `u64` | `ContractError::Overflow` (700) |

This guard is applied after all parameter checks.

---

## Validation Order

Checks are applied in this order so callers receive the most actionable error
first:

1. `amount > 0` → `InvalidBondAmount`
2. `duration > 0` → `InvalidBondDuration`
3. If `is_rolling`: `notice_period_duration > 0` and `<= duration` → `InvalidNoticePeriod`
4. `bond_start + duration` no overflow → `Overflow`

---

## Error Code Reference

| Error variant | Code | Category |
|---------------|------|----------|
| `InvalidBondAmount` | 214 | Bond |
| `InvalidBondDuration` | 215 | Bond |
| `InvalidNoticePeriod` | 216 | Bond |
| `Overflow` | 700 | Arithmetic |

All codes are wire-stable and must not be renumbered after deployment.

---

## Security Notes

- **No panics.** All invalid inputs return typed errors, preventing unexpected
  contract termination and making error handling predictable for callers.
- **Centralised amount rule.** `is_valid_bond` is the only place that defines
  `amount > 0`. `create_bond` calls it rather than duplicating the condition,
  ensuring the rule cannot diverge.
- **Rolling-bond notice invariant.** Enforcing `notice_period_duration <= duration`
  at creation time prevents a class of withdrawal-logic bugs where the notice
  window can never be satisfied, which could otherwise lock funds indefinitely.
- **Overflow guard.** The `bond_start + duration` check prevents silent wrap-
  around on the bond end timestamp, which could allow bonds to appear expired
  immediately after creation.
