//! INFObySolved: `extract::row_bound` keeps every row bound packable and a
//! superset of the rows a reference reads, for the anchor and for every family
//! member that shares the anchor's facts.
#![cfg(test)]

use super::*;
use crate::engine::authority::tests::wide_rows_order::boundary_rows;

const PACK_ABS_ROWS: i64 = 1 << 30;
const PACK_REL_ROWS: i64 = 1 << 29;

fn rows() -> Vec<u32> {
    let mut rows = boundary_rows();
    let max = u64::from(MAX_ROW);
    rows.extend(
        [
            (1u64 << 29) - 1,
            1 << 29,
            (1 << 29) + 7,
            (1 << 30) - 1,
            1 << 30,
        ]
        .into_iter()
        .filter(|&r| r <= max)
        .map(|r| r as u32),
    );
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn packable(b: Bound) -> bool {
    match b {
        Bound::Open => true,
        Bound::Abs(v) => i64::from(v) < PACK_ABS_ROWS,
        Bound::Rel(k) => i64::from(k).abs() < PACK_REL_ROWS,
    }
}

fn read_rows(v1: u32, abs: bool, anchor: u32) -> RefProj {
    let b = row_bound(Some(v1), abs, anchor, false);
    assert!(packable(b), "{v1} {abs} {anchor}");
    RefProj {
        sheet: 0,
        rows: AxisMap::point(b),
        cols: AxisMap::point(Bound::Rel(0)),
    }
}

#[test]
fn row_bounds_are_exact_when_packable_and_open_otherwise() {
    let rows = rows();
    for &placement in &rows {
        for &row in &rows {
            for abs in [false, true] {
                let b = row_bound(Some(row + 1), abs, placement, false);
                let diff = i64::from(row) - i64::from(placement);
                let exact = if abs {
                    i64::from(row) < PACK_ABS_ROWS
                } else {
                    diff.abs() < PACK_REL_ROWS
                };
                if !exact {
                    assert_eq!(b, Bound::Open, "{placement} {row} {abs}");
                } else if abs {
                    assert_eq!(b, Bound::Abs(row));
                } else {
                    assert_eq!(b, Bound::Rel(diff as i32));
                }
            }
        }
    }
    assert_eq!(row_bound(None, false, 3, false), Bound::Open);
    assert_eq!(row_bound(Some(0), true, 3, false), Bound::Open);
}

/// A family member's facts are its template's at the anchor
/// (`authority_host::authority_formula_input_shared`), so the anchor's row
/// bound, instantiated at any member, must still cover that member's own
/// target row.
#[test]
fn a_family_member_far_from_its_anchor_still_reads_its_target() {
    let max = i64::from(MAX_ROW);
    let rows = rows();
    for &anchor in &rows {
        for &target in &rows {
            let off = i64::from(target) - i64::from(anchor);
            let at_anchor = read_rows(target + 1, false, anchor);
            for step in [0i64, 1, 1000, 1 << 20] {
                let member = i64::from(anchor) + step;
                let member_target = member + off;
                if member > max || !(0..=max).contains(&member_target) {
                    continue;
                }
                let rect = at_anchor
                    .instantiate(member as u32, 0)
                    .expect("the anchor's read instantiates at the member");
                assert!(
                    i64::from(rect.r0) <= member_target && member_target <= i64::from(rect.r1),
                    "anchor {anchor} target {target} member {member}: {rect:?}"
                );
            }
        }
    }
}
