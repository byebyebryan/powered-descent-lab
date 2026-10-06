# Planner retirement and consolidation plan

Make policy 3 the sole maintained planner execution path, remove superseded
experiments after extracting their live dependencies, and finish evaluator and
report housekeeping. The starting checkpoint is `e1d9363`. The completed
[core loop cleanup](waypoint_v2_core_cleanup_results.md) remains closed.

## Maintained behavior

The current planner constructs a terrain-blind nominal transfer, audits its
fixed commands against real terrain, executes a local clearing correction when
appropriate, and replans from actual handoff H. Preserve the tested vehicle,
Earth gravity, 120 Hz physics and 60 Hz command envelope, finite search families,
ordering, ranking, policy parameters, commands, full state, fuel, attitude,
original deadline and command clock.

Preserve phase-aware admission and safety guards, consumed-prefix proof,
independent source replay, the private continuation certificate, partial error
evidence and pending-cycle checkpoint order. Queries and certificates do not
become live state. A useful clearing handoff needs finite safe continuation,
not a landing suffix or a mandatory terrain-feature far edge.

Terminal guidance, normal direct-transfer guidance and hand-authored waypoint
guidance remain supported. Ordinary `pd-cli run` and the optional `planner-v2`
CLI feature boundary remain unchanged. This batch does not add a per-tick host
controller, tune flight behavior or expand the terrain acceptance envelope.

## Retired behavior and historical reading

Retire V1 search and candidate exposure, V1-only generated-route execution,
the superseded fixed-gate ballistic experiment, V2 policy-1/2 execution,
superseded planner research commands and presentation generators, and frozen
pathwise/recoverability transfer scorers once maintained dependencies have been
extracted. Remove their dedicated implementation tests and runnable comparison
packs with an explicit inventory; the old test count is not an exit gate.

Known saved policy identities remain recognizable. Separate saved-record
validation from execution admission before changing the Rust policy default to
policy 3. Historical schemas, ordinary saved-action replay, persisted route
plans and necessary report readers do not require the historical search or
controller to remain executable. Retired execution requests must fail clearly,
not silently select a different policy or controller.

This authorization supersedes the previous structural-cleanup requirement to
retain historical executable APIs. It does not authorize rewriting historical
evidence. Keep frozen research input files, accepted captures, published reports,
selectors and receipts unchanged. No blanket deletion under `fixtures/research`
or `outputs` is permitted. Current V2 input expansion still uses research fixtures.

## Responsibility and sequencing

The primary owns architecture, retirement decisions, integration, acceptance,
documentation and reviewed local commits. Use one Luna Max implementation writer
by default for independently reviewable slices. Workers preserve unrelated edits
and do not commit, publish, push, restart servers or run new acceptance campaigns.

1. Establish a keep, extract and retire inventory covering commands, modules,
   public exports, packs, scripts and tests. Record protected evidence hashes.
2. Add a normal tracked policy-3 multi-handoff regression and a maintained
   validation runner with ordinary and explicit retained-capture modes.
3. Extract neutral exact-byte hashing, create-only JSON/byte writing and output
   reservation. Preserve newline/error behavior and caller-specific trust checks.
4. Extract live canonical initial generation/source fitting, airborne acquisition,
   terminal realization, phase/contact helpers and local-clearing execution/proof
   helpers from mixed historical research modules. Pure math stays in `pd-plan`;
   simulation-dependent realization stays in `pd-eval`. Introduce no framework
   or new standalone crate.
5. Retire old execution lanes in separate reviewable commits. Change policy
   defaults only after saved-version handling no longer depends on `Default`.
   Preserve serialized identities; remove obsolete exports and dependencies.
6. Split the surviving V2 pack into models, input expansion, execution/aggregation,
   provenance/capture validation, presentation and tests. Keep useful facade paths
   and persisted JSON identities, field ordering and report paths unchanged.
   Simplify remaining CLI arguments/dispatch only at cohesive boundaries.
7. Compute report summaries before moving sample/event/marker arrays into report
   data; remove their full-array clones. Retire superseded standalone presentation
   builders and separate reusable current checks from historical preview tools.
   Keep the common rich-detail and batch templates, plots, inspection controls,
   actual-H annotations and report navigation.
8. Reconcile current guidance, architecture, roadmap and entrypoint documentation;
   retain dated development history. Review the complete diff, record removals and
   verification results, and finish with a clean committed worktree.

## Validation

Focused tests run at each meaningful checkpoint. The ordinary regression suite
must cover policy-3 direct, corrected and multi-handoff behavior with tracked
inputs, state/fuel/clock continuity, original deadline, actual-H ownership,
integrity and source replay. External captures remain opt-in.

The numerical preservation gate compares all 44 complete flight records against:

`outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native/`

Exclude only `planning_s`, `execution_s` and `replay_s`. Commands, proposals,
states/contact, cycles, segments, manifests, outcomes and proof flags must match.
This is local regression execution, not new acceptance or publication evidence.
Run after integrating changes that could affect behavior, and again on final
source when relevant changes justify it. Retain and diagnose the first difference;
do not tune selection, alter inputs or expand exclusions to obtain parity.

The expected verdict remains 36 mandatory landings (11 direct and 25 corrected),
two diagnostic landings, four zero-step diagnostic stops and two unsupported
inputs; all 44 inputs pass integrity and all 42 supported results pass source
replay. Diagnostic outcomes remain separate from the landing denominator.

The final gate includes feature-enabled workspace and CLI tests, default-off
CLI help/dependency checks, formatting, diff whitespace, strict all-target Clippy,
all maintained JavaScript tests, rebuilt saved-capture acceptance and saved common
report checks. Retain the existing narrow `single_element_loop` lint allowance
only while its historical implementation remains; remove the allowance if that
implementation is retired. Render representative planner/terminal/transfer
reports into fresh temporary validation locations when necessary, never over
published pages. Test retired execution rejection and known historical identity
decoding independently.

Protect current inputs and saved evidence with before/after path/content hashes.
The published site must retain its 44 cases, 42 rich payloads, 36 actual handoffs
and 46 receipt-bound pages. Historical captures retain their original measured
source, not the source of this cleanup.

## Exit conditions

All approved retirement and consolidation items are implemented or have a
specific live dependency that requires a documented narrower retention, with
that dependency extracted where feasible. Current planner execution admits only
policy 3, historical data remains readable, all preservation and final gates
pass, intentional test/API removals are recorded, and no required migration
defect remains. Commit reviewed checkpoints as work proceeds.

No push, publication, server restart, new terrain campaign, solver redesign,
ordinary controller-default promotion or deletion of captured evidence is part
of this batch.

## Follow-up housekeeping round: October 5, 2026

The user authorized this round after a fresh read-only scan at clean `90a2228`,
using `worker-goal-loop` and reviewed local commits as work proceeds. The original
eight-phase batch above is already complete; this round removes its remaining
stranded dependencies rather than reopening the flight design.

1. Remove the uncalled strict/operational `pd-control::FlightProgram` playback
   module and execution-only core validators/factories. Preserve the live
   `FlightProgramUpdateV1` and historical binding/program serialized shapes and
   strict unknown-field handling; ordinary simulation/action replay is unchanged.
2. Remove V1 route authority, validation and shaping after removing unreachable
   new-run evaluator planner metrics and plan-artifact generation. Keep saved
   route/policy/diagnostic DTOs, their custom infinity decoding, historical cache
   completeness checks and saved-report refresh/rendering. Remove exclusive
   helpers/tests, not the current exact point-centred terrain/body query.
3. Retire execution of the four `single_dogleg_v1` packs and the `late_bend_v1`
   diagnostic pack, reject those profiles even in renamed packs, and remove only
   their exclusive geometry/tests. Keep pack JSON as archive metadata and saved
   reports; maintained bend/sequence guidance and legacy handoff envelopes stay.
4. Remove the unused sample-based aggregate-preview API. Extract the common rich
   template into an owned module without changing its bytes, charts, data,
   annotations or public maintained render/write entrypoints. Avoid unrelated
   evaluator-wide structural moves.
5. Reconcile nonexistent code paths, stale research-wrapper comments and
   active-versus-archival navigation labels; record the exact removals and gates.

The primary owns retirement/compatibility decisions, integration, final review,
documentation and commits. Luna writers handle bounded independent slices.
Workers do not commit or mutate fixtures/captured evidence. Keep the flight
invariants and final gates from the original plan: exact all-44 flight parity
excluding only three existing timing fields, ordinary workspace/CLI/default-off,
formatting, Clippy and maintained JS checks, saved-site acceptance and saved
report compatibility. No tuning or new acceptance campaign is permitted.

A fresh protected inventory is stored locally at
`outputs/validation/planner_housekeeping_round2_20261005/baseline.json`. Its 31
scopes include the original protected outputs plus all pack/scenario fixtures and
the old analytical fixtures; hashes also bind directory and symlink identities.
Compare against that inventory on final source. Published pages, current selectors
and receipts must remain unchanged; navigation-copy changes are source changes,
not authorization to republish the site.

This follow-up is complete at Rust checkpoint `f9e85b0`; its detailed retirement
inventory and final gates are recorded in the
[follow-up results](planner_retirement_cleanup_results.md#follow-up-housekeeping-round-october-5-2026).

## Small report-check and aggregation follow-up: October 5, 2026

This separately bounded round starts at clean `3d6d6c0`, following another
read-only scan and explicit `worker-goal-loop` authorization. It does not reopen
the completed retirement inventory or flight design.

1. Remove the five obsolete JavaScript report-check helpers whose only remaining
   consumers are their own tests, together with exclusive HTML parsing glue.
   Retain exact flight comparison, projection and handoff checks. Move useful
   assertions to the maintained common-template and saved-execution checks;
   retire tests for the old rebased navigation and preview-summary formats.
2. Summarize selected borrowed batch records without cloning whole records.
   Preserve summary serialization, numeric operation order, grouping/sorting,
   outcome classification and preferred comparison-lane behavior. Count refreshed
   V2 cases through their typed optional physical outcomes, rather than
   serializing the complete batch just to inspect those fields.

One Luna implementation writer owns both cohesive slices; the primary owns
review, integration, documentation and the reviewed local commit. Larger domain
module splits and reference-trajectory math consolidation are out of scope.
No report template, navigation, planner, controller, input or captured source
provenance changes are permitted.

Run focused JavaScript and aggregation regressions, then the maintained ordinary
developer gate and rebuilt evaluator's read-only saved-site checker. This slice
does not change flight computation, so the external all-44 flight-execution parity
check is not required again. Verify the same 31 protected scopes / 3,867 entries
before and after. Do not publish, push, restart servers or run a new campaign.

This small follow-up is complete; see the
[results and final gates](planner_retirement_cleanup_results.md#small-report-check-and-aggregation-follow-up-october-5-2026).
