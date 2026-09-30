//! INFObySolved: `event_key` orders as the `(column, row, kind)` tuple at the
//! `wide-rows` boundary rows.
#![cfg(test)]

use super::*;
use crate::engine::authority::tests::wide_rows_order::boundary_rows;

#[test]
fn event_key_orders_like_its_tuple() {
    let rows = boundary_rows();
    let mut tuples = Vec::new();
    for column in [0usize, 1, 16_383, 1 << 20] {
        for &row in &rows {
            for kind in 0..4u64 {
                tuples.push((column, row, kind));
            }
        }
    }
    let mut by_key = tuples.clone();
    by_key.sort_by_key(|&(c, r, k)| event_key(c, r, k));
    let mut by_tuple = tuples;
    by_tuple.sort_unstable();
    assert_eq!(by_key, by_tuple);
}
