# NameTree deletion planning in Rust

## Status

Proposed Phase 7 slice. Awaiting design review before implementation.

## Objective

Move the behavior that selects an indexed NameTree entry and plans the
resulting empty-node pruning and `/Limits` repairs into Rust. Keep PDF object
ownership and every structural or string mutation in C++.

The public API, static-library contract, deletion result, tree shape, stored
values, save output, malformed-input side effects, and resource bounds remain
unchanged.

## Considered boundaries

Three boundaries were considered:

1. Reuse Rust indexed lookup only, then retain all cleanup traversal in C++.
   This is low risk but moves almost no additional behavior.
2. Produce one fully validated Rust deletion plan, then apply it in C++.
   This moves target, pruning, and limit-repair decisions while preserving
   native ownership and an all-C++ fallback.
3. Let Rust invoke mutating callbacks while it traverses. This moves more
   orchestration but can leave a partially mutated tree if a later callback
   fails, making fallback unsafe.

The proposed boundary is option 2.

## Ownership boundary

Rust owns:

- depth-first global pair-index traversal in child order;
- selection of the target leaf and zero-based pair position;
- the historical 32-level traversal bound for target and cleanup searches;
- reconstruction of the root-to-leaf path, including child indices;
- prediction of which now-empty leaf or branch nodes are removed;
- selection of leaf-name or child-limit sources for repaired lower and upper
  limits after virtual removals;
- the decision that no limit repair is needed when the deleted name was not a
  normalized boundary;
- checked index and output-plan arithmetic.

C++ retains:

- dictionaries, arrays, strings, values, references, and their lifetimes;
- decoded `WideString` comparison and extraction;
- historical `/Limits` normalization for every node visited before the target,
  including trimming, appending missing entries, and swapping inverted bounds;
- validation and materialization of every planned source string before the
  first destructive write;
- removal of the key/value pair and empty children;
- all `/Limits` string writes;
- the complete original indexed search and recursive cleanup as the separately
  selected oracle and fallback.

No Rust allocation or pointer escapes across the FFI boundary. Rust receives
opaque handles and uses synchronous borrowed callbacks only.

## Planner contract

The planner returns one of three outcomes:

1. `missing`: the global pair index does not resolve to a direct value and the
   public deletion returns false;
2. `delete`: a target leaf/pair plus a bottom-up list of validated child-removal
   and optional limit-repair actions;
3. `unavailable`: a callback, validation, depth, or checked-arithmetic failure
   prevents a safe plan, so C++ runs the unchanged oracle from the original
   key/value state.

A deletion plan contains only opaque node handles, pair or child indices, and
typed references to existing leaf keys or child-limit entries. C++ resolves
all referenced strings and validates that every parent still owns the expected
child and every leaf still owns the expected pair before removing anything.

The target key and direct value are retained by C++ during validation. This
preserves exact lifetime and return behavior while allowing all plan inputs to
remain borrowed.

## Traversal and normalization parity

Target selection preserves the existing indexed traversal: a node with a
`/Names` array is treated as a leaf even when it also has `/Kids`; missing
children are skipped; direct or shared nodes are not globally deduplicated;
and recursion beyond level 32 cannot produce a target.

Cleanup planning reproduces the existing depth-first search for the selected
leaf. The C++ node-description callback normalizes `/Limits` before returning
comparisons, so nodes visited in earlier unsuccessful branches retain the same
historical normalization side effects. If planning later becomes unavailable,
the original C++ cleanup safely repeats those idempotent normalizations.

## Virtual cleanup plan

Rust computes the complete plan before the key/value removal:

1. Treat the target pair as absent when evaluating its leaf.
2. If the leaf becomes empty, mark it removable by its parent.
3. Walk ancestors bottom-up, virtually excluding each removed child.
4. Mark a branch removable when its `/Kids` array becomes empty.
5. When a surviving node has limits and the deleted key matched its normalized
   lower or upper bound, select the minimum and maximum remaining leaf keys or
   child limits as replacement sources.
6. When the deleted key matched neither boundary, preserve the normalized
   limits without a write.

The root itself is never removed. An empty root leaf or root `/Kids` array is
left in the same shape as the current implementation.

## Validation and application

C++ validates the entire plan before mutation:

- target leaf, pair bounds, key text, and direct value still match;
- every recorded parent/child relationship and child index still matches;
- every planned empty-node removal is still justified;
- every selected lower/upper source exists and can be materialized;
- every limit-write target still owns a mutable `/Limits` array.

After validation, C++ applies actions in the historical order: remove target
value then key, repair the target leaf if it survives, and walk bottom-up to
remove empty children and repair surviving ancestor limits. No callback into
Rust occurs during destructive application.

Validation or planning failure before removal invokes the unchanged C++
oracle. There is no fallback after destructive application starts because all
failure-capable reads have already completed.

## Complexity and resource behavior

Valid planning remains O(nodes + entries) in time. Rust uses O(depth) path and
action storage, bounded by the existing 32-level contract. Source selection
may scan the same leaf keys or child limits as the current cleanup. The slice
adds no persistent shadow tree and does not make deletion, saving, or limit
updates more eager.

## Differential evidence

Parser-native tests cover:

- missing, first, middle, and last global indices;
- empty-leaf and cascading empty-branch pruning;
- lower-only, upper-only, both-boundary, and no-repair cases;
- inverted, missing, and oversized limit arrays through normalized callback
  results;
- a leaf that also has kids, missing children, child order, shared/direct
  nodes, callback failure, checked rejection, and the depth cutoff.

Same-process C++ unit tests build independent identical Oracle and Candidate
trees. They delete every valid index plus misses from empty, flat, and
three-level fixtures and compare the boolean result, complete key/value order,
surviving exact value handles within each tree, full node shape, all limits,
and follow-up lookups.

The public Attachment deletion/save/reload corpus runs under both selectors
and compares attachment count, names, values, deletion results, save success,
and reload state. The complete Phase 7 unit and `pdfium_all` gates must pass
before activation is recorded.

## Activation and rollback

The existing test-only Rust parser selector controls activation. Production
candidate mode applies a Rust plan only after full C++ validation. Selecting
the C++ implementation or receiving `unavailable` executes the unchanged
oracle. No public header, build flag, or persisted format changes.

## Explicit deferrals

- PDF object or string ownership transfer;
- direct mutation from Rust callbacks;
- NameTree balancing or restructuring;
- deletion by decoded name rather than global index;
- removal of the C++ oracle.
