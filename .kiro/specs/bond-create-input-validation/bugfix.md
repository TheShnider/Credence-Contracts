# Bugfix Requirements Document

## Introduction

`CredenceBond::create_bond` accepts four parameters — `amount`, `duration`,
`is_rolling`, and `notice_period_duration` — but performs almost no input
validation. The only guard currently in place is an overflow check on
`bond_start + duration`. As a result:

- A caller can create a bond with `amount = 0` or a negative amount, making the
  bond economically meaningless and bypassing the `is_valid_bond` helper that
  already encodes the `amount > 0` rule.
- A caller can create a bond with `duration = 0`, producing an instant-expiry
  bond and making all time-based logic (lock-up, early-exit, rolling withdrawal)
  nonsensical.
- For rolling bonds, a caller can set `notice_period_duration > duration` or
  `notice_period_duration = 0`, making the rolling-withdrawal notice window
  either impossible to satisfy or undefined.

The fix must add typed `ContractError` rejections for every invalid combination,
reuse the existing `is_valid_bond` logic, and preserve all currently valid
creation paths unchanged.

---

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN `amount` is zero THEN the system accepts the bond creation without error

1.2 WHEN `amount` is negative THEN the system accepts the bond creation without error

1.3 WHEN `duration` is zero THEN the system accepts the bond creation without error

1.4 WHEN `is_rolling` is `true` AND `notice_period_duration` is zero THEN the system accepts the bond creation without error

1.5 WHEN `is_rolling` is `true` AND `notice_period_duration` is greater than `duration` THEN the system accepts the bond creation without error, producing a rolling bond whose notice window can never be satisfied

1.6 WHEN `amount` is non-positive THEN the system ignores the existing `is_valid_bond` helper that encodes the `amount > 0` rule

### Expected Behavior (Correct)

2.1 WHEN `amount` is zero THEN the system SHALL return `Err(ContractError::InvalidBondAmount)` without creating the bond

2.2 WHEN `amount` is negative THEN the system SHALL return `Err(ContractError::InvalidBondAmount)` without creating the bond

2.3 WHEN `duration` is zero THEN the system SHALL return `Err(ContractError::InvalidBondDuration)` without creating the bond

2.4 WHEN `is_rolling` is `true` AND `notice_period_duration` is zero THEN the system SHALL return `Err(ContractError::InvalidNoticePeriod)` without creating the bond

2.5 WHEN `is_rolling` is `true` AND `notice_period_duration` is greater than `duration` THEN the system SHALL return `Err(ContractError::InvalidNoticePeriod)` without creating the bond

2.6 WHEN `amount` is non-positive THEN the system SHALL delegate the amount check to `is_valid_bond` (or equivalent centralised logic) so the rule is enforced from a single source of truth

### Unchanged Behavior (Regression Prevention)

3.1 WHEN `amount` is strictly positive AND `duration` is strictly positive AND `is_rolling` is `false` THEN the system SHALL CONTINUE TO create the bond successfully

3.2 WHEN `amount` is strictly positive AND `duration` is strictly positive AND `is_rolling` is `true` AND `notice_period_duration` is strictly positive AND `notice_period_duration` is less than or equal to `duration` THEN the system SHALL CONTINUE TO create the bond successfully

3.3 WHEN `bond_start + duration` would overflow THEN the system SHALL CONTINUE TO return `Err(ContractError::Overflow)` as it does today

3.4 WHEN `amount` equals `i128::MAX` (maximum positive value) THEN the system SHALL CONTINUE TO accept the bond creation

3.5 WHEN `notice_period_duration` equals `duration` exactly (boundary case for rolling bonds) THEN the system SHALL CONTINUE TO accept the bond creation

---

## Bug Condition Pseudocode

### Bug Condition Function

```pascal
FUNCTION isBugCondition(amount, duration, is_rolling, notice_period_duration)
  INPUT: amount: i128, duration: u64, is_rolling: bool, notice_period_duration: u64
  OUTPUT: boolean

  IF amount <= 0 THEN RETURN true
  IF duration = 0 THEN RETURN true
  IF is_rolling AND notice_period_duration = 0 THEN RETURN true
  IF is_rolling AND notice_period_duration > duration THEN RETURN true
  RETURN false
END FUNCTION
```

### Fix-Checking Property

```pascal
// Property: Fix Checking — all buggy inputs must be rejected
FOR ALL (amount, duration, is_rolling, notice_period_duration)
    WHERE isBugCondition(amount, duration, is_rolling, notice_period_duration) DO
  result ← create_bond'(amount, duration, is_rolling, notice_period_duration)
  ASSERT result IS Err(_)
END FOR
```

### Preservation Property

```pascal
// Property: Preservation Checking — valid inputs must still succeed
FOR ALL (amount, duration, is_rolling, notice_period_duration)
    WHERE NOT isBugCondition(amount, duration, is_rolling, notice_period_duration) DO
  ASSERT create_bond(amount, duration, is_rolling, notice_period_duration)
       = create_bond'(amount, duration, is_rolling, notice_period_duration)
END FOR
```
