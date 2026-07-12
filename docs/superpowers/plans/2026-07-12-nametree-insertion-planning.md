# NameTree insertion planning implementation plan

## Goal

Implement the approved NameTree insertion-planning boundary from
`docs/superpowers/specs/2026-07-12-nametree-insertion-planning-design.md` in
small, independently reviewable commits while retaining the C++ oracle and
fallback.

## Commit 1: Rust planner and adapter boundary

- Add a typed Rust outcome for `duplicate` and `insert(node, pair_position)`.
- Preserve insertion-mode traversal order, object-number admission, the
  32-level depth bound, empty-root behavior, and leftmost-leaf selection.
- Add synchronous callback-based FFI and C++ adapter result types.
- Invoke the planner only for the selected Rust candidate and validate the
  returned node and position before any write.
- Fall back to the unchanged C++ oracle on planner or validation failure.
- Add parser-native tests for positions, duplicates, limits, traversal order,
  direct and repeated objects, callback failure, and depth rejection.

## Commit 2: Differential mutation evidence

- Build independent but identical C++ Oracle and Candidate trees.
- Compare insertion results, complete key/value order, exact inserted handles
  within each tree, affected limits, pair counts, and follow-up lookups.
- Run the public Attachment mutation/save/reload corpus under both selectors
  and compare the observable results.

## Commit 3: Activation evidence and documentation

- Run the parser-native suite, focused NameTree and Attachment tests, the full
  unit suite, and `pdfium_all`.
- Refresh the Rust/C++ ownership and size metrics.
- Record the slice, validation counts, and rollback boundary in the migration
  ledger and Draft PR description.

## Completion rule

The slice is complete only when all three commits are published, the Draft PR
records the intermediate evidence, and candidate mode remains clean with the
C++ oracle selectable and available as fallback.
