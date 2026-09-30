//! INFObySolved: tuple-oracle check of the row radix at the `wide-rows`
//! boundaries, including the exclusive end `MAX_ROW + 1`.
#![cfg(test)]

use super::*;
use crate::engine::authority::tests::wide_rows_order::boundary_rows;

fn events(n: usize) -> Vec<Event> {
    let mut rows = boundary_rows();
    rows.push(super::super::geom::MAX_ROW + 1);
    (0..n)
        .map(|i| Event {
            row: rows[(i * 7 + 3) % rows.len()],
            edge: i,
            start: i % 2 == 0,
        })
        .collect()
}

#[test]
fn radix_path_is_a_stable_row_sort() {
    let mut ev = events(300);
    let mut want: Vec<(u32, usize, bool)> = ev.iter().map(|e| (e.row, e.edge, e.start)).collect();
    want.sort_by_key(|&(row, _, _)| row);
    let mut temp = vec![Event::default(); ev.len()];
    sort_events(&mut ev, &mut temp, &mut RefinementWork::default());
    let got: Vec<_> = ev.iter().map(|e| (e.row, e.edge, e.start)).collect();
    assert_eq!(got, want);
}

#[test]
fn short_path_orders_rows() {
    let mut ev = events(40);
    let mut want: Vec<u32> = ev.iter().map(|e| e.row).collect();
    want.sort_unstable();
    let mut temp = vec![Event::default(); ev.len()];
    sort_events(&mut ev, &mut temp, &mut RefinementWork::default());
    let got: Vec<u32> = ev.iter().map(|e| e.row).collect();
    assert_eq!(got, want);
}
