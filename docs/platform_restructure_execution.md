# Unattended platform restructure run

[Architecture plan](platform_restructure_plan.md) · [Design review](platform_restructure_review.md) · [Roadmap](roadmap.md#7-recommended-immediate-next-step)

Status: implementation goal started on October 10, 2026. See the
[run record](platform_restructure_results.md) for accepted milestones and evidence.
This replaces the small-first-slice stopping point with a substantial
goal that can continue across accepted milestones while the owner is away.
The architecture and evidence boundaries in the linked plan still apply.

## Goal and completion

The implementation goal is a usable common platform for independent approaches:
neutral episode execution, self-contained procedural missions and collections,
generic evaluation and rich saved reports, an independent policy integration,
and correctly owned maintained controller/planner implementations.

The intended capacity is roughly a day of unattended work. Continue from an
accepted milestone to the next without requesting routine design confirmation.
Small reviewable changes are integration checkpoints within this one goal, not
separate goals that stop after the first wrapper. Finish early if the objective
is achieved; do not fill time with repeated checks or idle work.

The goal is complete only when all of these conditions hold:

1. An independent consumer can generate a mission, execute its own approach,
   save native commands, verify authoritative outcomes/replay and produce the
   maintained rich report through a documented workflow. Its normal dependency
   tree contains no current controller or planner, including with all features.
2. Procedural generation and collection management work without a Pylander
   checkout, retained outputs or solver feedback. Versioned recipes, structural
   rejection, immutable resolved inputs and collection usage are implemented.
3. Online policies and prepared programs use the shared host. Summary execution
   and captured execution preserve physical semantics; reset/step interfaces
   support an independent policy adapter without reward logic in the plant.
4. Maintained solver orchestration belongs to the solver implementation, with
   complete configuration. Generic evaluation and rendering have no executable
   strategy dependency. Temporary application-owned solver modules have been
   extracted or deliberately retired with caller/test mapping. Read-only
   historical decoding may remain.
5. Supported existing workflows retain commands, outcomes, serialized
   compatibility and proof/replay behavior. The maintained gate and relevant
   preservation checks pass; rich templates and retained evidence survive.
6. Current architecture/workflow docs and a run record describe what was
   implemented, measured, deferred and preserved. The checkout contains no
   unexplained changes or partially wired public paths.

A useful independent-agent milestone is a major deliverable even if later
solver migration exhausts the run budget. Report it precisely, but do not mark
the full restructure complete merely because that milestone or the day ends.

## Work order

The numbered stages in the architecture plan describe responsibilities. For
this run, prioritize a complete independent-agent workflow before the most
expensive legacy solver extraction.

| Milestone | Work | Acceptance before proceeding |
| --- | --- | --- |
| A Baseline | Record dirty ownership, source/input identities, maintained commands, baseline checks and retained reference availability | Preservation inventory and attributable baseline failures |
| B Runtime | Add `pd-run`, neutral records and one existing-controller adapter over core execution | Independent trivial agent; exact paired traces; time-zero, chunk and zero-command boundaries |
| C Missions | Add `pd-missions`, a versioned complete-mission distribution and collection tools | Deterministic generation, validation, usage roles and coverage characterization without solver filtering |
| D Evaluation and reports | Introduce neutral evaluation and report projections; separate application registration and historical readers | Independent approach evaluated and rendered with no strategy dependency; existing workflows still compile and verify |
| E Independent policy adapter | Add reset/step, capture modes, a small checkpoint-backed policy example and throughput measurements | Captured/summary physical equivalence; command replay; explicit observation/action mapping and completion reasons |
| F Maintained solvers | Extract Policy 3, then the combined-safety reference and required comparator into `pd-planner` | Separate exact preservation checks; complete configurations; no recovery or guard tuning |
| G Consolidation | Remove proven superseded execution paths, close dependency exceptions and update current docs/tooling | Caller/test migration accounted for; final maintained gate and acceptance inventory |

Mission work does not depend on extracting the existing planner. Checkpoint E
proves integration, not trained competence. Substantial RL training and a new
landing-performance campaign remain separate experimental work after the
platform contract is established.

## Transitional ownership

The current `pd-eval` library and binary both depend on concrete strategies;
the [existing binary](../pd-eval-cli/src/main.rs) registers them directly. Neutral
evaluation cannot depend on the old application to gain compatibility.

At milestone D, move the built-in application and any not-yet-extracted solver
modules to the proposed `pd-eval-cli` composition package. Its temporary
`legacy` area may retain their existing internal calls and tests while `pd-eval`
becomes neutral. Map every temporary module to its destination and removal
milestone in the run record. This is an intermediate state, not a claim that
solver ownership has been cleaned up. Milestone F moves maintained algorithms
to `pd-planner`; milestone G closes the remaining execution exceptions.

Retain the installed `pd-eval` binary name. Update Cargo package callers,
feature-gated CLI imports, test discovery and workflow examples together with
that split. Do not depend on a re-export through the application from the neutral
library or add a hidden built-in feature to make independence tests pass.
Historical decoding can remain isolated; it must not require executable
strategies or admit retired policies for execution.

## Decisions the loop may make

After the implementation goal is launched, routine choices are part of the
authorized work: module names, Rust signatures, configuration versions, new
generator parameters, test organization, compatibility adapters and integration
order within these dependencies. Resolve them using the contracts and measured
evidence, record material decisions, and keep progressing.

Declare the first generator distribution before inspecting controller results.
Use the current vehicle, Earth gravity and 120/60 Hz cadence first. Procedural
terrain, travel distance/direction and pad geometry must vary; preserve full
terrain between locally prepared pads. Source-pad rest is the initial tier;
airborne starts and resource variation are separate declared tiers when added.
The loop chooses explicit ranges and rejection limits under a new version and
characterizes them; it does not wait for a separate parameter approval.

Finite structural rejection, numerical portability fixes under a new generator
version, and typed unsupported capability are legitimate implementation choices.
Filtering by a solver's landing outcome, extending old mission budgets,
relaxing guards or changing old serialized identities are outside this refactor.
Do not use an existing controller as a fallback inside the independent example.

Work remains in this checkout. Preserve unrelated changes by their original
hunks and content, including untracked files. When a required integration edit
shares a dirty file, make a narrow extension and verify the original change is
still present; do not restore the file to Git HEAD. Existing navigation/server
logic should need no redesign for this work.

The run permits local code, docs, tests, builds, new mission inputs and bounded
validation artifacts. The owner's subsequent instruction, "commit as you go,"
authorizes scoped commits of accepted milestones. It does not authorize pushes, publication,
accepted-selector changes, server lifecycle changes, deletion of retained
outputs, a new worktree or delegation. Later explicit user instructions can
expand that scope.

## Validation budget

Use inexpensive contract checks first, then the maintained gate at integrated
milestones. Rerun a passed check when a relevant change or new failure justifies
it. Large repeated campaigns are not the default validation strategy.

Run the terrain tooling's pure/synthetic Python checks when migrating its
callers, readers or solver frontdoors; the ordinary developer gate does not
cover those tests. Commands and evidence scope remain in
[development](development.md) and [evaluation](evaluation.md).

| Check | Initial allowance and purpose |
| --- | --- |
| Generator characterization | 10,000 deterministic proposals, input-only; report accepted/rejected/exhausted counts, dimension coverage, duplicate content and repeat/order equivalence |
| Independent-agent integration | A sealed 64-mission collection from declared strata, plus explicit repeat cases; test execution, accounting and replay, not a landing-rate target |
| Existing controller preservation | Focused deterministic pairs over existing maintained cases; retain command/state/event/clock/fuel outcomes and legacy descriptor encoding |
| Policy 3 preservation | Explicit opt-in retained 44-case parity when flight execution changes; keep core and diagnostic groups distinct |
| Ballistic solver preservation | A fixed portable panel chosen from existing controls and stop mechanisms before moving that solver; compare current-source baseline and migrated execution exactly |
| Runtime throughput | Bounded command traces and repeatable batches; record actual steps, forecast work, capture/proof cost, wall time and memory where measurable |

The 10,000 proposals provide more input sampling than the old 1k collections;
they are not 10,000 verified landings. The 64 integration missions are consumed
development inputs after this pass, not untouched performance evaluation. Keep
repeat attempts separate from unique-mission counts and retain every scheduled
attempt's disposition.

These are initial bounds, not permissions to retry failed seeds indefinitely.
Increase a diagnostic allowance only to answer a specific unresolved contract
question, declare the change before new observations, and retain the original
counts. No landing-score search or full old/new 1k campaign belongs to this run.

The known retained parity path is
`outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native`. It exists
at preparation time; availability and identity are rechecked at launch. Follow
the explicit command in [development](development.md#optional-retained-flight-regression).
Do not substitute the older published source for a known exact comparator.
If unavailable or failing on the initial checkout, record the gap and establish
a fixed current-source paired baseline before changing the affected path;
ordinary tests still require no retained outputs.

Fresh artifacts use a unique create-only run directory under
`outputs/restructure/`. Only new demonstration reports may be rendered there.
Do not refresh existing reports or use a capture command's default publication
behavior. In particular, the existing native `run-pack` path publishes by
default; any justified invocation uses its explicit no-publication mode. Build
and saved-source proof commands are distinct from publication.

Freeze the relevant source throughout each proof capture. Keep progress logs
outside that captured source and update current docs between captures. An
unexpected concurrent source change invalidates that capture's integrity; keep
the attempt and diagnose the writer rather than retrying silently.

## Checkpoints and recovery

At launch, create a source/ownership inventory and a new run record. Before a
wide move, preserve the owned files or patch state needed to undo that move
without losing the starting dirty tree. Each accepted milestone records changed
files, decisions, checks, dependency exceptions, measured limits and next work.

Choose the next smallest change that advances an unmet completion condition.
After checking it, review the dependency boundary and artifact compatibility,
then continue to the next milestone. Update the run record at milestones and
before context handoff so a resumed loop does not restart the review or repeat
completed campaigns. Record command/log paths and protected identities.

A repeat failure needs a new diagnosis, not another unchanged invocation. If
three materially different fixes fail or an issue consumes roughly an hour
without new evidence, record that branch and work on another independent unmet
condition. Return only with a new hypothesis or prerequisite. This is an
anti-stall rule, not permission to bypass a failed preservation check.

Keep the newest accepted integration runnable. A change that cannot be brought
back to an accepted boundary is reverted only within the run's owned edits,
with its diagnosis retained. Never use broad reset/clean or rewrite fixtures to
recover a green gate. Pre-existing failures are recorded separately from new
regressions; neither is silently reported as a passing complete gate.

Defer dependent work when the only next action requires changing frozen
contracts, overwriting another writer's changes, publication or algorithm
tuning. Continue independent authorized work while recording that decision
boundary. A status update or milestone is not a reason to end the overall goal.

## Run completion record

Finish with one results document linked from the docs home and roadmap. It
contains the implemented ownership/dependency map, runnable independent-agent
workflow, generator versions and characterization, exact preservation scope,
passed checks, inherited gaps, and every remaining migration item.

If time or execution budget ends before the whole goal is achieved, leave the
checkout at the last accepted boundary and report completed milestones plus
unfinished work. Do not label a stub as trained RL, summary outcomes as verified
landings, a local report as publication, or a partial restructure as completion.
