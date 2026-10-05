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
