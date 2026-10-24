//! Overflow-checked arithmetic that stays cheap on SBF.
//!
//! `u64::checked_mul` lowers to a 128-bit multiply helper on SBF, which costs
//! about a hundred compute units per call. [`mul`] detects overflow with one
//! native division instead, and skips even that when both operands fit in
//! 32 bits.

/// `a * b`, or `None` on overflow.
#[inline(always)]
pub fn mul(a: u64, b: u64) -> Option<u64> {
    if (a | b) >> 32 == 0 {
        return Some(a.wrapping_mul(b));
    }
    if a == 0 {
        return Some(0);
    }
    let p = a.wrapping_mul(b);
    if p / a == b {
        Some(p)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::mul;

    #[test]
    fn matches_checked_mul() {
        let edges = [
            0,
            1,
            2,
            3,
            u32::MAX as u64,
            u32::MAX as u64 + 1,
            1 << 63,
            u64::MAX / 2,
            u64::MAX - 1,
            u64::MAX,
        ];
        for &a in &edges {
            for &b in &edges {
                assert_eq!(mul(a, b), a.checked_mul(b), "{a} * {b}");
            }
        }
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..100_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let a = x >> (x % 64);
            let b = x.rotate_left(17) >> (x.rotate_left(5) % 64);
            assert_eq!(mul(a, b), a.checked_mul(b));
        }
    }
}
