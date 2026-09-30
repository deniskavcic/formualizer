//! INFObySolved: tuple-oracle ordering of the authority's packed sort keys
//! at the row boundaries `wide-rows` crosses (2^20, 2^24, 2^31, `MAX_ROW`),
//! with high sheet ids, the last column and equal-key stability. Under the
//! default layout the rows above `MAX_ROW` are skipped, so the same tests
//! pin upstream's 20-bit behaviour.

use super::arc_sweep::check;
use super::support::Formula;
use crate::engine::authority::arc_sweep::{Probe, Slice};
use crate::engine::authority::candidates::discover;
use crate::engine::authority::geom::{Cover, MAX_COL, MAX_ROW, Rect, SYMBOL_SHEET};
use crate::engine::authority::plan_schedule::schedule;
use crate::engine::authority::planner::OrderedCell;
use crate::engine::authority::store::Store;
use crate::engine::vertex::VertexId;

/// Boundary rows valid under the active layout, ascending, distinct: byte
/// edges, then the rows `wide-rows` crosses (2^20, 2^24, 2^31, `MAX_ROW`).
/// Shared by every `wide-rows` ordering test (`tests/wide_rows/*`).
pub(crate) fn boundary_rows() -> Vec<u32> {
    let mut rows: Vec<u32> = [
        0u64,
        1,
        255,
        256,
        65_535,
        65_536,
        (1 << 20) - 1,
        1 << 20,
        (1 << 20) + 1,
        (1 << 24) - 1,
        1 << 24,
        (1 << 24) + 1,
        (1 << 31) - 1,
        1 << 31,
        (1 << 31) + 1,
        u64::from(MAX_ROW) - 1,
        u64::from(MAX_ROW),
    ]
    .into_iter()
    .filter(|&r| r <= u64::from(MAX_ROW))
    .map(|r| r as u32)
    .collect();
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn sheets() -> [u16; 4] {
    [0, 1, 0x1234, SYMBOL_SHEET - 1]
}

fn facts(l: u32) -> crate::engine::authority::store::FormulaFacts {
    Formula {
        refs: vec![],
        l: l.into(),
        literal: 0,
    }
    .facts()
}

fn check_discovery(cells: Vec<(u16, u32, u32)>) {
    // Alternate templates so vertically adjacent boundary rows stay separate
    // owners (every slice is then a single cell).
    let inputs = cells
        .iter()
        .enumerate()
        .map(|(i, &(sheet, row, col))| ((sheet, row, col), facts(i as u32 % 2)))
        .collect();
    let store = Store::build(inputs);
    let mut cover = Cover::new();
    for sheet in sheets() {
        cover.insert_rect(sheet, &Rect::new(0, 0, MAX_ROW, MAX_COL));
    }
    let out = discover(&store, &cover, None).unwrap();
    let mut got = Vec::new();
    for s in &out.slices {
        for row in s.r0..=s.r1 {
            got.push((s.sheet, s.col, row));
        }
    }
    let mut want: Vec<_> = cells.iter().map(|&(s, r, c)| (s, c, r)).collect();
    want.sort_unstable();
    assert_eq!(got, want);
    assert_eq!(out.cells as usize, want.len());
}

#[test]
fn candidate_slices_follow_sheet_column_row_order_at_wide_row_boundaries() {
    let rows = boundary_rows();
    let mut cells = Vec::new();
    // Reverse input order so the sort, not insertion, produces the order.
    for sheet in sheets().into_iter().rev() {
        for col in [MAX_COL, 1, 0] {
            for &row in rows.iter().rev() {
                cells.push((sheet, row, col));
            }
        }
    }
    // Radix path (more than SMALL_DISCOVERY = 32 cells).
    assert!(cells.len() > 32);
    check_discovery(cells.clone());
    // Comparison path (a handful of cells) must order identically.
    let small: Vec<_> = cells.iter().copied().step_by(cells.len() / 20).collect();
    assert!(small.len() <= 32);
    check_discovery(small);
}

fn run_schedule(input: &[OrderedCell]) {
    let out = schedule(
        input,
        0,
        None,
        |c| Ok(VertexId::new(c.id + 1_000)),
        |_| Ok(()),
    )
    .unwrap();
    // Tuple oracle, stable: (layer, sheet, column, row).
    let mut want = input.to_vec();
    want.sort_by_key(|c| (c.layer, c.sheet, c.col, c.row));
    assert_eq!(out.entries.len(), want.len());
    for (e, w) in out.entries.iter().zip(&want) {
        assert_eq!(
            (e.cell.sheet, e.cell.col, e.cell.row, e.cell.id),
            (w.sheet, w.col, w.row, w.id)
        );
    }
}

#[test]
fn schedule_member_order_follows_tuple_oracle_at_wide_row_boundaries() {
    let rows = boundary_rows();
    let mut input = Vec::new();
    let mut id = 0u32;
    for sheet in sheets().into_iter().rev() {
        for col in [MAX_COL, 0] {
            for &row in rows.iter().rev() {
                input.push(OrderedCell {
                    sheet,
                    row,
                    col,
                    id,
                    owner: id,
                    layer: u64::from(id % 2),
                    cycle: None,
                    chain: false,
                });
                id += 1;
            }
        }
    }
    // Cell-sort path.
    run_schedule(&input);

    // Run-sort path (at least RUN_SORT_MIN = 4096 cells): runs of one
    // owner that straddle each boundary row, emitted in reverse order.
    let mut input = Vec::new();
    let mut id = 0u32;
    let starts: Vec<u32> = rows
        .iter()
        .map(|&r| r.saturating_sub(100).min(MAX_ROW - 199))
        .collect();
    let mut owner = 0u32;
    for sheet in [SYMBOL_SHEET - 1, 7] {
        for col in [MAX_COL, 3] {
            let mut last_end = None;
            for &r0 in starts.iter() {
                // Keep runs disjoint when boundary rows are close together.
                let r0 = last_end.map_or(r0, |e: u32| r0.max(e + 1));
                if r0 > MAX_ROW - 199 {
                    continue;
                }
                last_end = Some(r0 + 199);
                owner += 1;
                for r in (r0..r0 + 200).rev() {
                    input.push(OrderedCell {
                        sheet,
                        row: r,
                        col,
                        id,
                        owner,
                        layer: 4,
                        cycle: None,
                        chain: false,
                    });
                    id += 1;
                }
            }
        }
    }
    input.reverse();
    if input.len() < 4096 {
        // Pad with a far-away block so the run path is exercised.
        let base = 5_000u32;
        for r in base..base + (4096 - input.len() as u32) {
            input.push(OrderedCell {
                sheet: 2,
                row: r,
                col: 9,
                id,
                owner: u32::MAX,
                layer: 4,
                cycle: None,
                chain: false,
            });
            id += 1;
        }
    }
    assert!(input.len() >= 4096);
    run_schedule(&input);
}

/// Slices and probes at the boundary rows on high sheet ids and the last
/// column; the brute force in `arc_sweep::check` is the oracle.
#[test]
fn arc_sweep_matches_brute_force_at_wide_row_boundaries() {
    let rows = boundary_rows();
    let mut slices = Vec::new();
    for sheet in [0u16, 0x7ffe] {
        for col in [0, MAX_COL] {
            for &r in &rows {
                slices.push(Slice {
                    sheet,
                    col,
                    r0: r,
                    r1: r,
                });
            }
        }
    }
    let mut probes = Vec::new();
    for (reader, &r) in rows.iter().enumerate() {
        for sheet in [0u16, 0x7ffe] {
            probes.push(Probe {
                reader: reader % slices.len(),
                sheet,
                image: Rect::cell(r, MAX_COL),
            });
            probes.push(Probe {
                reader: (reader + 1) % slices.len(),
                sheet,
                image: Rect::new(r, 0, MAX_ROW, 0),
            });
            probes.push(Probe {
                reader: (reader + 2) % slices.len(),
                sheet,
                image: Rect::new(0, 0, r, MAX_COL),
            });
        }
    }
    check(&slices, &probes);
}
