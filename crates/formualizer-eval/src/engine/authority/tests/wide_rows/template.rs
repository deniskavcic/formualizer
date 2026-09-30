//! INFObySolved: relative axis tokens stay injective across `wide-rows`
//! offsets (33 signed bits) and match upstream for non-negative offsets.
#![cfg(test)]

use super::axis;

#[test]
fn relative_axis_tokens_are_injective() {
    assert_eq!(axis(11, false, 0), (2 << 62) | 10);
    let far = u32::MAX;
    // Offsets exactly 2^32 apart used to truncate to one token.
    let a = axis(1, false, far - 1); // offset -(2^32 - 1)
    let b = axis(far, false, 0); // offset 2^32 - 2
    let c = axis(2, false, far - 1); // offset -(2^32 - 2)
    assert_ne!(a, axis(2, false, 0)); // offset 1 = a's offset + 2^32
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(axis(1, false, 0), axis(1, false, 1));
    assert_ne!(axis(5, true, 0), axis(5, false, 0));
}
