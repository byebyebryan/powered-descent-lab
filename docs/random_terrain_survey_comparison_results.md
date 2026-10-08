# Comparison-aware random-terrain sweep — corrected-control preservation stop

[Documentation home](README.md) · [Authorized follow-up](random_terrain_survey_recheck_plan.md#comparison-aware-follow-up-authorized-2026-10-06) · [Previous recheck](random_terrain_survey_recheck_results.md)

## Verdict

The requested audit-clearance comparison fix is implemented consistently in
Python collection/verification and Rust report verification. Readiness checks
passed, and a new sweep goal started with the original frozen inputs.
The direct preservation control now passes with one explicitly logged exception.
The sweep then stopped at the corrected control: it also landed with passing
integrity/replay, but seven selected-trajectory clearance diagnostics changed
their final digits, changing the content-derived proposal identity and its
segment reference. The two-field comparison rule does not permit those changes.

The selected maneuver, commands, states, actual handoff, physical outcome and
proofs are unchanged. This is another comparison-contract stop, not a new
flight crash. The readiness review missed the selected-trajectory/hash dependency.
No comparison was widened after measuring, and no retry or extra flight ran.
All 100 random cases, including the original failed `random-001`, remain
unattempted in this capture. Broad procedural reliability is still unmeasured.

## Attempt accounting

| Cohort | Frozen allowance | Attempted | Protocol-recorded | Unattempted |
| --- | ---: | ---: | ---: | ---: |
| Preservation controls | 3 | 2 | 1 | 1 |
| Random primary cases | 100 | 0 | 0 | 100 |
| Predeclared repeats | 5 | 0 | 0 | 5 |
| Total | 108 | 2 | 1 | 106 |

`v2_clear_685` is `recorded`: direct target landing, zero corrections, clear
nominal, integrity/replay passing. Its one audit minimum-clearance exception
is +8.881784197001252e-16 m, recorded with its exact path and both values.

`v2_ridge_early` is `evidence_error`: native exit zero, target landing,
one correction, blocked nominal, integrity/replay passing. The exact comparator
stopped at `/cycles/0/local_search/selected/identity`. Its successful native
flight tuple is not a successful cross-source preservation comparison.
The third control, checkpoint, random population and repeats did not run.

## Complete corrected-control diagnosis

Excluding only the three predeclared wall-clock timings, recursive comparison
finds **ten changed values**, and no other non-timing differences:

| Changed data | Count | Difference |
| --- | ---: | --- |
| Selected trajectory `body_clearance_m` | 7 | +1.4210854715202004e-14 or +2.842170943040401e-14 m |
| Cycle 1 audit minimum clearance | 1 | +8.881784197001252e-16 m |
| Selected proposal identity and segment reference | 2 | Different hashes of the changed diagnostic contents |

The trace offsets are 0, 2, 26, 31, 35, 616 and 619. All 1203 selected
trajectory states, scheduled commands, boundaries and every other proposal
field are exact. Both sources choose `source_62_5_percent_row_26`, template
row 26: upright target attitude, acceleration factor 1, 720 powered ticks.
Entry/powered-end/handoff/continuation steps remain 1272/1992/2234/2474.
Actual handoff position remains (-605.9442976371221, 340.4203888346855) m.

The identity calculation in
[`proposal_identity`](../pd-eval/src/local_clearing.rs) clears the proposal's
identity field, serializes the entire proposal and hashes it with FNV1a64.
The proposal includes every selected trajectory's `body_clearance_m`, so the
seven changed diagnostic values necessarily affect that content identity.
The segment retains the corresponding proposal identity for proof binding.

Read-only diagnosis independently recomputed both saved identities from their
canonical serialized proposal contents, preserving native numeric lexemes:

- Accepted source: `fnv1a64:625fb6b5b71915d7` — hash matches its own payload.
- Recheck source: `fnv1a64:de3707ef5411b67a` — hash matches its own payload.

The complete differences show no new maneuver or physical deviation. However,
the approved `derived_clearance_preservation_v1` contract allows only the two
cycle-audit clearance scalar families, not selected trace diagnostics or
derived identities. Correctly honoring that rule requires this stop.

## Fix and readiness review

One tracked [comparison contract](../studies/terrain_profiles/sentinel_comparison.json)
is shared by Python and Rust. Cross-source controls allow at most 1e-12 m only
in `minimum_airborne.clearance_m` and `first_violation.clearance_m` under cycle
audit scans. Metadata, required reserve, zero/required-reserve decisions and
all other non-timing fields stay exact. Exceptions are logged and recomputed
by both saved verifiers. The new manifest/input-tool copy binds the contract
to the measured source. Absent means historical exact comparison; old captures
are not automatically upgraded. Same-source repeats remain exact apart from
the three declared flight wall timings.

The readiness review also tightened Python's boolean-versus-number comparison,
finite/nonnegative timing exclusion checks, exception receipt comparison and
completed-capture source-before/after checks. These are artifact/validator
changes, not planner/controller or runtime safety tolerance changes.

Before freezing, all 41 terrain-study/survey Python tests passed. Both languages
pass the same 30 comparison conformance cases, including threshold crossings,
changed command/state/handoff/proof, missing/non-finite data, modified contracts
and larger differences. Synthetic runner tests verify sentinel exception
logging and strict repeats. The full eleven-step developer gate passed:
215 evaluator tests passed (four deliberately ignored), workspace/CLI boundaries,
formatting, strict Clippy, 52 maintained JavaScript tests and documentation.
Both older captures continue to verify under their original rules.

The remaining readiness gap is now explicit: body-envelope diagnostics are
also serialized in selected local trajectories and included in content hashes.
Core contact/foot/hull fields and state snapshots have different ownership and
must not receive a broad `clearance`-name wildcard exemption.

## Inputs, provenance and report

The new capture is
[`capture-random-20261006-v3-comparison`](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v3-comparison/survey.json).
Before measuring, all 100 raw files, 103 scenarios, plan and seed-list bytes
matched the original reviewed capture. All 100 native input-only preflights
and the reversed fresh-process generation check passed. Current vehicle,
Earth gravity, 120/60 Hz clocks, 90 s horizon, terrain recipe and local pad
patches were unchanged. There was no cutaway, tuning or changed seed.

The source freeze binds 118 files plus the executable at base commit
`ffc59ec28613b35327c23edaeae0b586d4cdf085` and saved uncommitted snapshots:

- Executable SHA-256: `537531805872b36ab3e0476a363c1c0ed59e44d4c174f95ae29b42e71ea794f5`
- Comparator contract SHA-256: `c830d46d187b82fa955ca047e90a4eac530f58b24718e8269151ef86ef89375e`
- Capture receipt SHA-256: `e56ea570289e1c0fd3b5308b65ef2eddfed0159f25a8684a5a5d0e12ed7418f5`
- Raw repeat SHA-256: `05a3b02abbea033d905c1e04757a073022ac923cf1ab897eae88470a36660b9c`

The separate [common-template batch](../outputs/reports/eval/planner_v2_random_terrain/recheck-20261006-v3/index.html)
has one rich accepted control detail and 107 failed/unattempted status pages,
actual attempted/unattempted counts and the audited exception count. The
corrected control's [raw flight](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v3-comparison/runs/v2_ridge_early/flight.json)
and logs remain linked without claiming preservation acceptance.
Normal report navigation includes the new batch and retains both older stopped
reports. The accepted 44-case site/selector is not promoted or regenerated.

Saved verification executes zero fresh flights/replays. A preservation receipt
protects 274 original files, including both earlier stopped report sites/capture
summaries/receipts and the accepted benchmark. Server changes, commits and
pushes were not included in this pass.

Final checks authenticate all three survey captures, the current frozen source
and executable, and all 274 protected file hashes. Four navigation tests pass.
The common report check verifies one rich payload with zero actual handoff
annotations, 107 status pages, all 108 detail destinations, 695 local links
across 112 pages and normal navigation to all three development captures.
These are automated/static checks, not human browser acceptance. The report
server remains stopped and no commit or push was performed.

## Next decision

The next comparator revision needs explicit approval; no additional campaign
is authorized by this results document. Avoid another single-field patch
followed immediately by measurement:

1. Finish an implementation-based inventory of changed body-envelope evidence
   and all identity/reference consumers, using the retained direct/single/multiple
   controls and the two measured corrected-control payloads. No new flights are
   needed for that readiness work.
2. Permit bounded selected-trajectory `body_clearance_m` roundoff only with exact
   states, clocks, commands, selected row/template and safety decisions. Keep the
   same small bound; do not touch core contact metrics or widen runtime reserves.
3. Independently validate each source's proposal content hash and all corresponding
   segment/cycle references. Compare the selected semantic payload exactly except
   named diagnostics. Do not simply ignore all identities or change runtime hash
   definitions to retain a historical bit pattern.
4. Test the observed full corrected pair and the identity-binding rejection cases
   before freezing. Retain exact same-source repeats and all old capture rules.
5. Only after that review/validation, authorize a new create-only same-input
   bounded sweep. Preserve this stopped root and its measured source unchanged.

The scalar comparison fix works for the direct control; its current scope is
incomplete for corrected controls. The original source-rest numerical issue
still needs the procedural failed-case end-to-end recheck.
