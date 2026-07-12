# NameTree insertion planning in Rust

## Status

Approved implementation slice within Phase 7 of the Rust-majority migration.
This design narrows the approved Variant 1 boundary to insertion planning only.

## Objective

Move the behavior that decides whether a NameTree insertion is a duplicate and,
if not, which leaf and sorted pair position receive the new entry into Rust.
Keep PDF object ownership and every mutation in C++.

The public API, static-library contract, tree shape, stored values, save output,
and resource bounds remain unchanged.

## Ownership boundary

Rust owns:

- insertion-mode NameTree traversal in child order;
- exact-name duplicate detection;
- selection of the target leaf;
- selection of a zero-based pair insertion position;
- the historical 32-level traversal bound;
- exact indirect object-number cycle admission order for `/Names`, its items,
  `/Limits`, its items, `/Kids`, and child dictionaries;
- the decision to use the leftmost leaf when the query sorts before every
  existing entry.

C++ retains:

- dictionaries, arrays, strings, values, references, and their lifetimes;
- decoded `WideString` comparison;
- object-number extraction;
- the historical insertion-mode `/Limits` normalization, including trimming
  to two items, appending missing items, and swapping inverted bounds;
- insertion of the key and value into the selected array;
- ancestor discovery and lower/upper limit updates;
- the complete original insertion search as the separately selected oracle
  and fallback.

No Rust allocation or pointer escapes across the FFI boundary. Rust receives
opaque handles and uses synchronous borrowed callbacks only.

## Planner contract

The planner returns one of two completed outcomes:

1. `duplicate`: the exact key already exists and `AddValueAndName()` returns
   false without inserting;
2. `insert`: a target leaf handle and a zero-based pair position where C++
   inserts the key followed by its value.

Separately, `unavailable` means no safe plan can be produced, so C++ runs the
unchanged oracle path.

An empty root `/Names` array is an `insert` result at pair position zero.
Within a leaf, the first greater key defines the insertion position, an equal
key is a duplicate, and a key greater than all admitted entries appends after
the last pair. If the query sorts before every admitted leaf, Rust selects the
leftmost reachable leaf at pair position zero.

The planner preserves the current global object-number seen set and its
admission order. Direct objects with object number zero are never treated as
cycles. A callback failure, invalid returned handle, checked-arithmetic
failure, or resource-bound rejection produces `unavailable`; C++ performs no
Rust-selected write in that case.

## Data flow

`CPDF_NameTree::AddValueAndName()` invokes the Rust planner only when the Rust
parser candidate is selected. C++ callbacks expose node shape, array/object
tokens, normalized limit comparisons, leaf-key comparisons, and children.
Rust returns only the outcome, target node, and pair position. C++ validates
that the node still owns a `/Names` array and that the position is in range
before inserting.

After insertion, the existing C++ ancestor walk updates all applicable
`/Limits` arrays exactly as before. Candidate rejection or validation failure
falls back to `SearchNameNodeByName()` and the original mutation path.

## Complexity and resource behavior

The valid path remains O(nodes + entries) in expected time. Rust uses O(depth)
traversal stack plus O(indirect objects) visited-set storage. The maximum
candidate recursion depth remains 32, matching the existing NameTree contract.
The slice does not make tree balancing, mutation, or save behavior more eager.

## Differential evidence

Parser-native tests cover:

- empty-root insertion;
- prepend, middle, and append positions;
- exact duplicates;
- lower/upper limits and inverted or oversized limit arrays;
- leftmost-leaf fallback;
- child order;
- direct objects and repeated indirect object numbers;
- callback failure and the depth cutoff.

Same-process C++ unit tests build independent but identical Oracle and
Candidate trees because insertion mutates the object graph. They compare the
boolean result, complete key/value order, exact value handles within each tree,
all affected `/Limits`, pair count, and subsequent lookup results across the
existing empty, flat, and three-level insertion corpus.

The public Attachment mutation corpus runs under both implementations and
compares attachment count, names, values, save success, and reload results.
The complete Phase 7 unit and `pdfium_all` gates must pass before activation is
recorded.

## Activation and rollback

The existing test-only Rust parser selector controls activation. Production
candidate mode uses the Rust plan only after all callbacks and C++ validation
succeed. Selecting the C++ implementation or receiving `unavailable` executes
the unchanged oracle. No public header, build flag, or persisted format changes.

## Explicit deferrals

- key/value insertion and ownership transfer;
- ancestor discovery and `/Limits` writes after insertion;
- NameTree deletion planning or mutation;
- tree balancing or restructuring;
- removal of the C++ oracle.
