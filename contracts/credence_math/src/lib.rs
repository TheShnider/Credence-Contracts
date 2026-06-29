#![no_std]
#![allow(
    deprecated,
    unused_imports,
    unused_variables,
    dead_code,
    unused_assignments,
    unused_mut,
    mismatched_lifetime_syntaxes,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::restriction
)]

use credence_errors::ContractError;
use ethnum::U256;

/// Fixed-point denominator for basis-point calculations.
pub const BPS_DENOMINATOR: i128 = 10_000;

/// Rounding behavior for [`mul_div_i128`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rounding {
    /// Truncate the fractional remainder toward zero.
    Down,
    /// Round away from zero when the division leaves any remainder.
    Up,
    /// Round to the nearest integer, with exact half-way cases rounded away from zero.
    Nearest,
}

/// Checked `u64` multiplication with a stable panic message.
#[inline]
#[must_use]
pub fn mul_u64(a: u64, b: u64, msg: &'static str) -> u64 {
    a.checked_mul(b).unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` addition with a stable panic message.
#[inline]
#[must_use]
pub fn add_i128(a: i128, b: i128, msg: &'static str) -> i128 {
    a.checked_add(b).unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` subtraction with a stable panic message.
#[inline]
#[must_use]
pub fn sub_i128(a: i128, b: i128, msg: &'static str) -> i128 {
    a.checked_sub(b).unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` multiplication with a stable panic message.
#[inline]
#[must_use]
pub fn mul_i128(a: i128, b: i128, msg: &'static str) -> i128 {
    a.checked_mul(b).unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` division with a stable panic message.
#[inline]
#[must_use]
pub fn div_i128(a: i128, b: i128, msg: &'static str) -> i128 {
    a.checked_div(b).unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` ceiling division with a stable panic message.
/// Computes ceil(a / b) for b > 0, a >= 0.
///
/// # Panics
/// Panics with `msg` on `b == 0` (via the inner `checked_add(b - 1)` /
/// `checked_div`). Prefer [`ceil_div_checked_i128`] on hot paths where
/// `b == 0` is reachable so callers receive a typed
/// [`ContractError::DivisionByZero`] instead of a string panic.
#[inline]
#[must_use]
pub fn ceil_div_i128(a: i128, b: i128, msg: &'static str) -> i128 {
    a.checked_add(b - 1)
        .unwrap_or_else(|| panic!("{msg}"))
        .checked_div(b)
        .unwrap_or_else(|| panic!("{msg}"))
}

/// Checked `i128` division returning a typed error instead of panicking.
///
/// Returns [`ContractError::DivisionByZero`] when `b == 0`, and
/// [`ContractError::Overflow`] for the single overflowing case
/// `i128::MIN / -1`. Otherwise returns `a / b` (truncated toward zero,
/// matching Rust integer division).
///
/// Prefer this over [`div_i128`] on paths where a zero denominator is a
/// reachable runtime state (e.g. a fully-slashed bond) so the fault maps to
/// a wire-stable Arithmetic error code rather than a free-form panic string.
///
/// # Examples
///
/// ```
/// use credence_math::div_checked_i128;
/// use credence_errors::ContractError;
///
/// assert_eq!(div_checked_i128(10, 3), Ok(3));
/// assert_eq!(div_checked_i128(7, 0), Err(ContractError::DivisionByZero));
/// ```
#[inline]
pub fn div_checked_i128(a: i128, b: i128) -> Result<i128, ContractError> {
    if b == 0 {
        return Err(ContractError::DivisionByZero);
    }
    a.checked_div(b).ok_or(ContractError::Overflow)
}

/// Checked `i128` ceiling division returning a typed error instead of panicking.
///
/// Computes `ceil(a / b)` for `b > 0`, `a >= 0`. The `b == 0` case is rejected
/// **before** the `b - 1` subtraction so a zero denominator surfaces as
/// [`ContractError::DivisionByZero`] rather than being masked as an
/// [`ContractError::Overflow`] from the subtraction. Returns
/// [`ContractError::Overflow`] if the intermediate `a + (b - 1)` overflows.
///
/// This is the typed counterpart to [`ceil_div_i128`] used on the slash-percentage
/// hot path `ceil(slashed * 10_000 / bonded)`, where `bonded == 0` is reachable
/// for a fully-slashed bond.
///
/// # Examples
///
/// ```
/// use credence_math::ceil_div_checked_i128;
/// use credence_errors::ContractError;
///
/// // bonded = 3, slashed = 2: ceil(2 * 10_000 / 3) = 6667
/// assert_eq!(ceil_div_checked_i128(2 * 10_000, 3), Ok(6667));
/// assert_eq!(ceil_div_checked_i128(10, 5), Ok(2));
/// assert_eq!(ceil_div_checked_i128(0, 5), Ok(0));
/// // b == 0 is rejected before `b - 1`, so it is DivisionByZero, not Overflow.
/// assert_eq!(ceil_div_checked_i128(5, 0), Err(ContractError::DivisionByZero));
/// ```
#[inline]
pub fn ceil_div_checked_i128(a: i128, b: i128) -> Result<i128, ContractError> {
    if b == 0 {
        return Err(ContractError::DivisionByZero);
    }
    a.checked_add(b - 1)
        .ok_or(ContractError::Overflow)?
        .checked_div(b)
        .ok_or(ContractError::Overflow)
}

/// Compute `a * b / denom` over a 256-bit intermediate.
///
/// The intermediate product is widened before division, so large products that
/// exceed `i128` can still succeed when the final rounded result fits in
/// `i128`. `Rounding::Down` matches Rust integer division by truncating toward
/// zero. `Rounding::Up` rounds away from zero on any remainder.
/// `Rounding::Nearest` rounds to the nearest integer, with half-way cases
/// rounded away from zero.
///
/// # Panics
///
/// Panics with `msg` if `denom` is zero or if the final rounded result does not
/// fit in `i128`.
///
/// # Examples
///
/// ```
/// use credence_math::{mul_div_i128, Rounding};
///
/// assert_eq!(mul_div_i128(i128::MAX, 10_000, 10_000, Rounding::Down, "overflow"), i128::MAX);
/// assert_eq!(mul_div_i128(10, 3, 4, Rounding::Down, "overflow"), 7);
/// assert_eq!(mul_div_i128(10, 3, 4, Rounding::Up, "overflow"), 8);
/// assert_eq!(mul_div_i128(10, 3, 4, Rounding::Nearest, "overflow"), 8);
/// assert_eq!(mul_div_i128(-10, 3, 4, Rounding::Up, "overflow"), -8);
/// ```
#[inline]
#[must_use]
pub fn mul_div_i128(a: i128, b: i128, denom: i128, mode: Rounding, msg: &'static str) -> i128 {
    if denom == 0 {
        panic!("{msg}");
    }

    let negative = (a < 0) ^ (b < 0) ^ (denom < 0);
    let numerator = U256::new(a.unsigned_abs()) * U256::new(b.unsigned_abs());
    let divisor = U256::new(denom.unsigned_abs());
    let quotient = numerator / divisor;
    let remainder = numerator % divisor;

    let rounded = match mode {
        Rounding::Down => quotient,
        Rounding::Up => {
            if remainder == U256::ZERO {
                quotient
            } else {
                quotient + U256::ONE
            }
        }
        Rounding::Nearest => {
            if remainder * U256::new(2) >= divisor {
                quotient + U256::ONE
            } else {
                quotient
            }
        }
    };

    let positive_limit = U256::new(i128::MAX as u128);
    let negative_limit = U256::new((i128::MAX as u128) + 1);
    if negative {
        if rounded > negative_limit {
            panic!("{msg}");
        }
        if rounded == negative_limit {
            i128::MIN
        } else {
            -i128::try_from(rounded.as_u128()).unwrap_or_else(|_| panic!("{msg}"))
        }
    } else {
        if rounded > positive_limit {
            panic!("{msg}");
        }
        i128::try_from(rounded.as_u128()).unwrap_or_else(|_| panic!("{msg}"))
    }
}

/// Calculate a basis-point percentage of an `i128` amount: `amount * bps / BPS_DENOMINATOR`.
#[inline]
#[must_use]
pub fn bps(amount: i128, bps: u32, mul_msg: &'static str, div_msg: &'static str) -> i128 {
    let numerator = mul_i128(amount, bps as i128, mul_msg);
    div_i128(numerator, BPS_DENOMINATOR, div_msg)
}

/// Calculate a basis-point percentage of an `i128` amount, rounded away from zero.
///
/// Uses [`mul_div_i128`] so `amount * bps` cannot overflow before division.
///
/// # Examples
///
/// ```
/// use credence_math::bps_round_up;
///
/// assert_eq!(bps_round_up(10_001, 1, "overflow"), 2);
/// assert_eq!(bps_round_up(10_000, 1, "overflow"), 1);
/// assert_eq!(bps_round_up(-10_001, 1, "overflow"), -2);
/// ```
#[inline]
#[must_use]
pub fn bps_round_up(amount: i128, bps_value: u32, msg: &'static str) -> i128 {
    mul_div_i128(
        amount,
        bps_value as i128,
        BPS_DENOMINATOR,
        Rounding::Up,
        msg,
    )
}

/// Calculate a basis-point percentage of a `u64` amount: `amount * bps / BPS_DENOMINATOR`.
#[inline]
#[must_use]
pub fn bps_u64(amount: u64, bps: u32, mul_msg: &'static str) -> u64 {
    mul_u64(amount, bps as u64, mul_msg) / BPS_DENOMINATOR as u64
}

/// Split an amount into `(fee, net)` using basis-point math.
#[inline]
#[must_use]
pub fn split_bps(
    amount: i128,
    bps_value: u32,
    mul_msg: &'static str,
    div_msg: &'static str,
    sub_msg: &'static str,
) -> (i128, i128) {
    let fee = bps(amount, bps_value, mul_msg, div_msg);
    let net = sub_i128(amount, fee, sub_msg);
    (fee, net)
}

#[cfg(test)]
mod tests {
    use super::{
        bps, bps_round_up, bps_u64, ceil_div_checked_i128, ceil_div_i128, div_checked_i128,
        mul_div_i128, split_bps, Rounding,
    };
    use credence_errors::ContractError;

    fn legacy_bps_i128(amount: i128, bps: u32) -> i128 {
        amount
            .checked_mul(bps as i128)
            .expect("legacy i128 overflow")
            / 10_000
    }

    fn legacy_bps_u64(amount: u64, bps: u32) -> u64 {
        amount.checked_mul(bps as u64).expect("legacy u64 overflow") / 10_000
    }

    fn legacy_split_bps(amount: i128, bps: u32) -> (i128, i128) {
        let fee = legacy_bps_i128(amount, bps);
        let net = amount.checked_sub(fee).expect("legacy i128 underflow");
        (fee, net)
    }

    #[test]
    fn bps_matches_legacy_formula() {
        let cases = [
            (0_i128, 0_u32),
            (1, 1),
            (10_000, 100),
            (999_999, 333),
            (1_000_000_000, 50),
            (i128::MAX / 20_000, 10_000),
        ];

        for (amount, bps_value) in cases {
            assert_eq!(
                bps(amount, bps_value, "mul", "div"),
                legacy_bps_i128(amount, bps_value)
            );
        }
    }

    #[test]
    fn mul_div_down_matches_legacy_bps_formula() {
        let cases = [
            (0_i128, 0_u32),
            (1, 1),
            (10_000, 100),
            (999_999, 333),
            (1_000_000_000, 50),
            (i128::MAX / 20_000, 10_000),
        ];

        for (amount, bps_value) in cases {
            assert_eq!(
                mul_div_i128(
                    amount,
                    bps_value as i128,
                    10_000,
                    Rounding::Down,
                    "overflow"
                ),
                legacy_bps_i128(amount, bps_value)
            );
        }
    }

    #[test]
    fn bps_u64_matches_legacy_formula() {
        let cases = [
            (0_u64, 0_u32),
            (1, 1),
            (10_000, 100),
            (999_999, 333),
            (u64::MAX / 20_000, 10_000),
        ];

        for (amount, bps_value) in cases {
            assert_eq!(
                bps_u64(amount, bps_value, "mul"),
                legacy_bps_u64(amount, bps_value)
            );
        }
    }

    #[test]
    fn split_bps_matches_legacy_formula() {
        let cases = [
            (0_i128, 0_u32),
            (10_000, 100),
            (10_000, 1_000),
            (123_456_789, 75),
            (i128::MAX / 20_000, 10_000),
        ];

        for (amount, bps_value) in cases {
            assert_eq!(
                split_bps(amount, bps_value, "mul", "div", "sub"),
                legacy_split_bps(amount, bps_value)
            );
        }
    }

    #[test]
    fn mul_div_down_matches_rust_division_for_signed_inputs() {
        assert_eq!(mul_div_i128(-10, 3, 4, Rounding::Down, "test"), -7);
        assert_eq!(mul_div_i128(10, -3, 4, Rounding::Down, "test"), -7);
        assert_eq!(mul_div_i128(10, 3, -4, Rounding::Down, "test"), -7);
        assert_eq!(mul_div_i128(-10, -3, -4, Rounding::Down, "test"), -7);
    }

    #[test]
    fn mul_div_uses_wide_intermediate_when_result_fits() {
        assert_eq!(
            mul_div_i128(i128::MAX, 10_000, 10_000, Rounding::Down, "test"),
            i128::MAX
        );
        assert_eq!(
            mul_div_i128(i128::MAX, 10_000, 10_000, Rounding::Up, "test"),
            i128::MAX
        );
    }

    #[test]
    fn mul_div_rounds_up_on_non_zero_remainder() {
        assert_eq!(mul_div_i128(10, 3, 4, Rounding::Down, "test"), 7);
        assert_eq!(mul_div_i128(10, 3, 4, Rounding::Up, "test"), 8);
        assert_eq!(mul_div_i128(-10, 3, 4, Rounding::Up, "test"), -8);
    }

    #[test]
    fn mul_div_nearest_rounds_half_ties_away_from_zero() {
        assert_eq!(mul_div_i128(10, 1, 4, Rounding::Nearest, "test"), 3);
        assert_eq!(mul_div_i128(9, 1, 4, Rounding::Nearest, "test"), 2);
        assert_eq!(mul_div_i128(-10, 1, 4, Rounding::Nearest, "test"), -3);
    }

    #[test]
    fn mul_div_handles_zero_numerator_and_denom_one() {
        assert_eq!(mul_div_i128(0, i128::MAX, 1, Rounding::Up, "test"), 0);
        assert_eq!(mul_div_i128(123, 456, 1, Rounding::Down, "test"), 56_088);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn mul_div_panics_only_when_final_positive_result_overflows() {
        let _ = mul_div_i128(i128::MAX, 10_001, 10_000, Rounding::Down, "overflow");
    }

    #[test]
    #[should_panic(expected = "denom")]
    fn mul_div_panics_with_msg_on_zero_denominator() {
        let _ = mul_div_i128(1, 1, 0, Rounding::Down, "denom");
    }

    #[test]
    fn bps_round_up_uses_wide_intermediate() {
        assert_eq!(bps_round_up(10_001, 1, "test"), 2);
        assert_eq!(bps_round_up(10_000, 1, "test"), 1);
        assert_eq!(bps_round_up(i128::MAX, 10_000, "test"), i128::MAX);
    }

    #[test]
    fn ceil_div_i128_zero_numerator() {
        assert_eq!(ceil_div_i128(0, 5, "test"), 0);
    }

    #[test]
    fn ceil_div_i128_exact_division() {
        assert_eq!(ceil_div_i128(10, 5, "test"), 2);
    }

    #[test]
    fn ceil_div_i128_off_by_one_boundary() {
        assert_eq!(ceil_div_i128(11, 5, "test"), 3);
    }

    #[test]
    fn ceil_div_i128_large_values() {
        assert_eq!(ceil_div_i128(10_000 * 5_001, 10_001, "test"), 5001);
    }

    #[test]
    fn ceil_div_i128_bonded_one() {
        assert_eq!(ceil_div_i128(0, 1, "test"), 0);
        assert_eq!(ceil_div_i128(1, 1, "test"), 1);
    }

    #[test]
    fn ceil_div_i128_known_pairs() {
        // bonded=3, slashed=2: ceil(2*10_000/3) = 6667
        assert_eq!(ceil_div_i128(2 * 10_000, 3, "test"), 6667);
        // bonded=7, slashed=3: ceil(3*10_000/7) = 4286
        assert_eq!(ceil_div_i128(3 * 10_000, 7, "test"), 4286);
    }

    // --- div_checked_i128 ---

    #[test]
    fn div_checked_rejects_zero_denominator() {
        assert_eq!(div_checked_i128(7, 0), Err(ContractError::DivisionByZero));
        assert_eq!(div_checked_i128(0, 0), Err(ContractError::DivisionByZero));
    }

    #[test]
    fn div_checked_basic_and_b_one() {
        assert_eq!(div_checked_i128(10, 3), Ok(3));
        assert_eq!(div_checked_i128(10, 1), Ok(10));
        assert_eq!(div_checked_i128(i128::MIN, 1), Ok(i128::MIN));
        assert_eq!(div_checked_i128(-10, 3), Ok(-3));
    }

    #[test]
    fn div_checked_min_div_neg_one_overflows() {
        // i128::MIN / -1 is the sole overflowing division; b != 0 so it must
        // surface as Overflow, never DivisionByZero.
        assert_eq!(div_checked_i128(i128::MIN, -1), Err(ContractError::Overflow));
    }

    // --- ceil_div_checked_i128 ---

    #[test]
    fn ceil_div_checked_rejects_zero_before_subtraction() {
        // b == 0 must be DivisionByZero, not Overflow masked by `b - 1`.
        assert_eq!(
            ceil_div_checked_i128(5, 0),
            Err(ContractError::DivisionByZero)
        );
        assert_eq!(
            ceil_div_checked_i128(i128::MIN, 0),
            Err(ContractError::DivisionByZero)
        );
    }

    #[test]
    fn ceil_div_checked_matches_panicking_helper() {
        let cases = [(0_i128, 5_i128), (10, 5), (11, 5), (1, 1), (0, 1)];
        for (a, b) in cases {
            assert_eq!(ceil_div_checked_i128(a, b), Ok(ceil_div_i128(a, b, "x")));
        }
        assert_eq!(ceil_div_checked_i128(2 * 10_000, 3), Ok(6667));
    }

    #[test]
    fn ceil_div_checked_intermediate_overflow() {
        // a + (b - 1) overflows i128 -> Overflow, not a panic.
        assert_eq!(
            ceil_div_checked_i128(i128::MAX, 2),
            Err(ContractError::Overflow)
        );
    }
}
