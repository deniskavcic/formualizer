# Solved fork (`solved-grid-v3`)

This file is ours. Upstream does not have it. It lists every intentional
divergence from Formualizer `main`, and why grid-v3 needs it. Update it in the
same commit as a fork change.

Upstream base: `dfd2f964` (v0.13.0, 2026-10-26). Everything below was carried
through that merge; our earlier commits are in the branch history.

Audit after a sync:

```bash
git diff dfd2f964 --stat -- crates
git grep -n INFObySolved
```

Every hunk we add to an upstream file is marked `// INFObySolved:` and points
at the grid-v3 module that needs it. With the `wide-rows` feature off, the
default layout matches upstream.

## Wide rows

`formualizer-common` feature `wide-rows` (off by default). Grid-v3 turns it on.
Excel's 20-bit row field becomes 32 bits: 4,294,967,296 rows by 16,384 columns.
`u32::MAX` stays reserved, so the last usable row is `u32::MAX - 1`
(`packing::GRID_ROW_MAX` in `formualizer-common/src/coord.rs`). Grid-v3 still
caps a sheet at 10,000,000 rows; the packing is what makes that possible.

The authority, address packing, spill guards, and `OFFSET` all derive their
limits from that module instead of restating Excel's widths. Under
`wide-rows`, `OFFSET` resolves out to the gated grid edge instead of `#REF!`
at Excel's 1,048,576 rows. The default layout is unchanged.

A row bound that does not fit the authority's 30-bit packed form widens to the
open grid edge (`engine/authority/extract.rs`). It can over-approximate a
dependency. It does not drop one. Our tests for this live in
`engine/authority/tests/wide_rows/` and `wide_rows_order.rs`, with a two-line
hook left in each upstream test module.

### Row limits added upstream in 0.11-0.13

Upstream's newer code restates Excel's height. Each site below follows the fork
instead, and the default layout still gets upstream's values. Grid-v3's tests
for them are in `server/rust/tests/wide_rows.rs`.

- `formualizer-parse/src/tokenizer.rs`, `is_a1_range_end`: the row limit and its
  digit count come from `GRID_ROW_MAX`. Otherwise `A1:A2000000` parses as two
  cells joined by `:`, depends only on those two cells, and goes stale when a
  cell inside it changes. The parse crate has no `wide-rows` feature of its own;
  it reads the limit from `formualizer-common`.
- `formualizer-eval/src/interpreter.rs`, open-ended `@`: `SHEET_ROWS` comes from
  `GRID_ROW_MAX`. Otherwise `=@A:A` past Excel's height returns `#VALUE!`.
- `formualizer-eval/src/builtins/stats/mod.rs`, `open_range_frame`: paired
  statistics (CORREL, SLOPE and the rest) size `A:A` by `eval.max_open_ended_rows`
  through a new `EvaluationContext::max_open_ended_rows` (`traits.rs`, with the
  `Engine`, `DynamicRefCollector` and `RecordingContext` impls). Otherwise
  `=CORREL(A:A,B1:B2000000)` returns `#N/A`. The value is per workbook and set at
  creation or hydrate, so a sheet grown past it reads short until the next
  hydrate. Other evaluation contexts keep upstream's 1,048,576.
- `formualizer-eval/src/engine/eval.rs`, `check_insert_room`: the sum is widened
  to `u64`. Under `wide-rows` the limit is `u32::MAX`, so upstream's saturating
  sum never exceeds it and an insert could push content off the last row without
  being refused. The 10,000,000-row product cap cannot reach this; `wide-rows`
  still claims the full range. Upstream's
  `engine/tests/insert_at_grid_edge.rs` takes its last row from the packing and
  expects `=SUM(A3:A)` there, because a range ending on the last row prints as
  open.

## Checkpoint hydrate

Two constructors on the Arrow store, both in
`formualizer-eval/src/arrow_store/mod.rs`, used by
`server/rust/src/sheet/persist`:

- `ColumnChunk::from_lanes` builds a compacted chunk from lane arrays.
- `ArrowSheet::from_prebuilt` rebuilds a sheet from those chunks.

Both start the overlays and the merged-lane cache empty, the same way an
upstream chunk does.

## Undo and structural edits

`b61c9ccc`, for grid-v3 undo:

- `ReferenceContext` and `ReferenceAdjuster::adjust_ast_if_changed_in_context`
  are `pub` instead of `pub(crate)` (`engine/graph/editor/reference_adjuster.rs`).
  Undo rebasing calls them from `server/rust/src/sheet/undo/rebase.rs`. The
  adjustment itself is unchanged.
- A deleted formula vertex is not written into the live cell map
  (`engine/graph/editor/vertex_editor.rs`). Otherwise a formula removed by a
  structural edit can reappear on a live cell. Covered by
  `server/rust/tests/structural.rs`.

## Appended columns stay unmaterialized

`ArrowSheet::insert_columns` (`crates/formualizer-eval/src/arrow_store/mod.rs`)
no longer copies column 0's chunk heights when it appends at the end of a
dense sheet. A grid-v3 write far to the right of the dense seed columns used
to give every new column a full-height empty chunk: 1045 MiB RSS at 65,536
rows, about 30.5 GiB extrapolated at 2,000,000 rows (measured by
`bench_max_bounds_in_memory` in `server/rust/tests/wide_rows.rs`). Appended
columns now start with no chunks; missing chunks already read as empty, and
`ensure_column_chunk_mut` materializes one chunk on the first write. Covered by
`appended_columns_stay_unmaterialized_until_written` in the same file.

An insert before an existing column on a dense-aligned sheet still copies the
chunk heights, and the empty-sheet arm is unchanged. After an append the sheet
is no longer dense-aligned (the new columns have no chunks), so later inserts
take upstream's sparse arm and stay unmaterialized too. Upstream's
`column_insert_delete_retains_chunk_alignment` and
`multiple_adjacent_column_ops_alignment` still pass.

Upstream check, 2026-11-03: v0.13.0 `dfd2f964` has no fix; `insert_columns` is
unchanged since `7ec3b5f1`.
