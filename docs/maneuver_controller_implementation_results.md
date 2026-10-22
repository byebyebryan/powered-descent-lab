# Maneuver controller software implementation

[Plan](maneuver_controller_implementation_plan.md) · [Controller design](maneuver_controller_design.md) · [Runnable example](../examples/maneuver-policy/README.md)

October 21, 2026. Implementation starts from pushed checkpoint `4448f08`.
The experimental library and deterministic native composition implement the
four planned software slices. This records software checks, not a controller
qualification, timing experiment or RL allocation.

## Implemented boundaries

- `pd-control::maneuver` defines typed requests, admissions, feedback, native
  end and `ManeuverController`. The terminal adapter binds one original context,
  coasts upright, resets once at acceptance and preserves backend commands and
  telemetry. Its descriptor carries a backend implementation/configuration pair,
  so alternate complete landing implementations need no terminal-specific type.
  No context replacement or durable restore operation exists.
- `pd-maneuver-example` owns one native Episode at 120/60/10 Hz. Immediate,
  never and fixed-tick programs see physical mission/observation and common
  feedback. Activation uses the same boundary; planner calls end at commitment.
- Captured execution uses ordinary neutral evaluation/seal files with separate
  versioned trace and receipt. Source hashes are compiled into the application
  and compared with live snapshots and executable identity. Create-only writes,
  independent reconstruction and bounded readers preserve evidence boundaries.
- The example has its own dependency guard. Neutral crates, the independent
  checkpoint example, existing CLI defaults and planner remain unchanged.

## Validation and review

The predeclared library panel derives eleven cases from the unchanged tracked
flat-terminal fixture: nominal, loaded (9,450 kg dry), lateral cleanup,
nonzero attitude, raised terrain, near-contact settling, low fuel, fuel
exhaustion, short deadline, ascending and below-target entry. Case definitions
and the exact comparator were written before evaluating the adapter. Direct
and wrapped runs use the same episode ID and omit timing on both sides;
complete frames, transitions and final captures must match with no physical
exclusions. All eleven comparisons pass, including failed flight outcomes.

Native source review corrected the plan's zero-initial-fuel case: native
validation requires positive initial fuel. The panel uses 0.008 kg so fuel
exhausts during execution, and unit tests preserve the invalid-input disposition
for a zero-fuel scenario. No plant validation or frozen fixture was changed.

Persistence review found the existing untagged telemetry decoder maps integer
metrics to floats. Saved reconstruction compares the neutral encode/decode
projection, while commands compare directly and exactly. Changed telemetry and
an adjacent representable command value fail. In-memory backend parity remains typed exact
equality. No neutral schema was changed to accommodate the adapter.

| Check | Result |
| --- | --- |
| Library unit tests | 181 pass, including four new contract/lifecycle tests |
| Native exact parity | Eleven declared cases pass in one regression test |
| First library checkpoint | All 18 maintained developer checks pass |
| Composition library | Ten tests pass: schedules, capture modes, failures, serialization, integrity, partial writes, independent tampering and bounded files |
| Real CLI | Two tests pass: single run/verify/rich-report/create-only path and unsupported/malformed inputs without fabricated capture |
| Final maintained gate | All 19 checks pass, including strict all-target Clippy, docs, Node, CLI/default/feature boundaries and the new composition dependency guard |

Only software tests use temporary synthetic episodes/bundles and HTML. No
training, terminal-pack campaign, report refresh/publication, selector change,
output retirement or server operation was performed. The existing report
templates are reused intact. Existing controller implementations, planner,
native runtime/evaluator, fixtures and historical experiment bytes are preserved.

The original implementation audit preserved all 785 unrelated pre-existing files byte-for-byte,
including pending research/report work. The 12 existing-file changes are the
workspace/package wiring, controller export, gate checks and scoped design/workflow
edits; 18 new files contain the implementation, tests and run record. Mixed
evaluation/report documents retain their original pending text. The real Git
index was empty; implementation awaited review. That review tree
resolves documentation links without relying on unrelated untracked files.

Local software evidence is retained at
`/tmp/pd-maneuver-implementation-_wy_p8ul`: `baseline.json`, the passing
`slice-one-gate.log` and `final-developer-gate-r2.log`, plus
`preservation-validation.json` and `implementation-source-inventory.json`.
The temporary location is a local review aid; checked-in tests and the maintained
gate supply fresh-checkout reproducibility.

## Pre-commit review

The review found one evidence-integrity defect: unchanged companion bytes and
reconstructable decisions could make composite verification pass despite the
neutral evaluation recording changed input files during execution. The new
`input_drift_fails_bundle_integrity_while_preserving_independent_replay` test
failed against the original verifier, then passed after bundle integrity was
made to require the neutral execution-integrity verdict. Physical replay and
controller/planner reconstruction remain independently available; a failed run
integrity verdict cannot produce a passing composite result.

The complete maintained gate passes all 19 checks after this fix, with ten
composition library tests and two CLI tests. Review evidence is retained at
`/tmp/pd-maneuver-review-z6rpak4a/developer-gate.log`. The reviewed commit packages
only the implementation, this fix and its workflow/design records; unrelated
pending research/report work is preserved. No controller tuning or measured
characterization was performed. The eleven-case comparator establishes exact
adapter parity, not eleven successful landings or a measured landing envelope.

## Remaining measurement boundary

The [controller evaluation design](controller_evaluation_design.md) specifies
the proposed populations, coverage, confirmation and activation comparisons.
It is design work, not a measured allocation.

There is no inherited landing envelope from adapter parity. Declare a cost
pilot and activation/envelope characterization separately, including independent
full-pack access, clean/trajectory-error cases, fresh/frontier roles and hard
resource limits. Later backends need their own identity and parity/qualification.
Learning, arbitrary arrival, retargeting and mid-maneuver restore remain future
design choices. Reconstruction requires the frozen source/executable at its
bound paths; physical command replay remains independent across builds.
