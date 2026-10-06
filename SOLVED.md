# Solved fork (`solved-grid-v3`)

This file is ours. Upstream does not have it. It lists every intentional
divergence from Formualizer `main`, and why grid-v3 needs it. Update it in the
same commit as a fork change.

Upstream base: `7ec3b5f1` (v0.10.1, 2026-09-30). Our commits on top of that
merge are `003a4f16` (wide-rows port) and `b61c9ccc` (reference adjuster and
deleted formula vertices).

Audit after a sync:

```bash
git diff 7ec3b5f1 --stat -- crates
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
