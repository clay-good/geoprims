<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# A5 parent and children (`indexing.a5.hierarchy`)

## Method

A5 0.10.1 stores the resolution marker and Hilbert path in the 64-bit index. Moving to a parent removes the final subdivision bits; moving to immediate children appends every valid next subdivision. Resolution 0 has five resolution 1 children, while later cells have four.

## Equations

From resolution 2 onward, one level multiplies the logical cell count by 4. The hierarchy is logical rather than a geometric partition: child pentagons overlap their parent but do not exactly tile its boundary.

## Symbols and units

Cell, parent, and children are 16-digit hexadecimal A5 indexes; resolution is an integer from 0 to 30.

## Domain

Bounded cells at resolutions 0–29. Resolution 30 has no finer children and is refused with a named input error.

## Approximations

None in the index operations. The hierarchy describes index locality, not exact polygon containment.

## Worked example

- sourcePublisher: A5 project
- sourceTitle: A5 hierarchy API
- sourceEdition: a5-js 0.10.1
- sourceLocator: cellToParent and cellToChildren for 0x63611a8000000000
- independent: no (canonical a5-js cross-language reference; the tool remains experimental)
- inputs: cell 63611a8000000000
- outputs: parent 63611a0000000000 and four immediate children
- tolerance: exact indexes
- verifiedBy: automated differential test
- verifiedOn: 2026-10-12

## Differential tests

- `core/crates/gp-indexing/tests/a5_parity.rs`: parent and every immediate child match a5-js 0.10.1 exactly for 120 cases spanning all resolutions where children exist.

## Invariants

- Every child's immediate parent is the input cell.
- A parent is exactly one resolution coarser; ordinary children are exactly one resolution finer.
