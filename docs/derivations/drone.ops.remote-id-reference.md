<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# FAA Remote ID reference (`drone.ops.remote-id-reference`)

## Method

Read the `faa-89-remote-id` row from the shared dated regulation file and return its status, compliance date, summary, citation, review date, and authority link. The tool makes no compliance decision.

## Equations

None. This is a versioned reference lookup.

## Symbols and units

The compliance date is an ISO date. The other outputs are text.

## Domain

No inputs. The build fails if the named regulation row is missing or malformed.

## Approximations

None. The result is current only through the review date shown in the notice and does not interpret equipment or operational exceptions.

## Worked example

- sourcePublisher: Federal Aviation Administration, via eCFR
- sourceTitle: 14 CFR Part 89 Remote ID
- sourceEdition: eCFR, reviewed 2026-09-18
- sourceLocator: Part 89, Remote Identification of Unmanned Aircraft
- independent: yes
- inputs: none
- outputs: status "In force" and compliance date 2024-03-16
- tolerance: exact text and date
- verifiedBy: golden vector v001 and `core/crates/gp-drone/tests/power_ops.rs` `dated_faa_references_keep_their_legal_status`
- verifiedOn: 2026-10-12

## Differential tests

Not applicable to an exact lookup. The golden vector is checked against the same public result returned through both runtime surfaces.

## Invariants

- Every operations-reference example carries the dated not-legal-advice notice.
- The result never contains a compliance verdict for a particular aircraft or operation.
