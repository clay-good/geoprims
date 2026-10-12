<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# FAA Part 108 proposed-rule reference (`drone.ops.part108-proposed-reference`)

## Method

Read the `faa-108-nprm` row from the shared dated regulation file and return its legal status, summary, Federal Register citation, review date, and authority link. The result deliberately does not apply any proposed provision as a compliance rule.

## Equations

None. This is a versioned reference lookup.

## Symbols and units

All outputs are text.

## Domain

No inputs. The build fails if the named regulation row is missing or malformed.

## Approximations

None. The summary is not a substitute for the full proposal, and the status is current only through the review date shown in the notice.

## Worked example

- sourcePublisher: Federal Aviation Administration and Transportation Security Administration
- sourceTitle: Normalizing Unmanned Aircraft Systems Beyond Visual Line of Sight Operations
- sourceEdition: Notice of proposed rulemaking, 90 FR 38212
- sourceLocator: Docket FAA-2025-1908, published August 7, 2025
- independent: yes
- inputs: none
- outputs: status "Proposed" and citation containing 90 FR 38212
- tolerance: exact text
- verifiedBy: golden vector v001 and `core/crates/gp-drone/tests/power_ops.rs` `dated_faa_references_keep_their_legal_status`
- verifiedOn: 2026-10-12

## Differential tests

Not applicable to an exact lookup. The golden vector is checked against the same public result returned through both runtime surfaces.

## Invariants

- The status is "Proposed," never "In force."
- The summary says the result is not an in-force compliance rule.
- Every operations-reference example carries the dated not-legal-advice notice.
