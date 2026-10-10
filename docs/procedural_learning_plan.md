# Procedural learning execution contract

[Documentation home](README.md) · [Platform](architecture.md) · [Mission collections](procedural_missions.md)

The owner launched this loop on October 10 with “go for a goal loop, commit as
you go.” Work remains in the shared checkout. The platform extraction is complete;
this loop must reach actual training and independent native evaluation.

## Goal and task definitions

Establish complete procedural landing missions as the primary benchmark, train
an independent RL approach, and measure generalization alongside the incumbent
using shared physical rules and evidence. A negative learning result is useful
only after the declared experiment and final evaluation have actually run.
An untrained checkpoint adapter is not completion.

Terminal landing and simple transfer are synthetic physical diagnostics.
Their neutral tasks specify initial state, target and physical/resource limits,
not boost/coast/handoff phases. Authored terrain, waypoint and planner probes
remain approach-specific diagnostics and regressions. Their intermediate
contracts do not become universal acceptance requirements for new policies.
Existing fixtures and historical outcomes are unchanged.

The primary task is source-pad-rest to designated-pad landing within the
declared fuel and scenario horizon. Core contact and mission semantics remain
authoritative. Rewards, waypoints, route representation and internal phases
cannot redefine success. The initial envelope retains the current vehicle,
Earth gravity and 120/60 Hz physics/action clocks. Mission terrain, direction,
distance, elevation and pad geometry vary without outcome-based filtering.

## Work order and ownership

1. Record current source, dirty-file ownership, task roles and this contract.
2. Extend `pd-missions` with a versioned layered terrain family grounded in the
   earlier profile studies alongside unchanged value-noise generation. Keep
   native generation independent of Pylander and saved flight captures. Verify
   generator computations against an independent reference before any flights.
3. Extend neutral evaluation with campaign-level provenance, per-input binding,
   attempt accounting, incumbent adapters and common report projections. Remove
   whole-collection work from each episode; do not blindly cache mutable evidence.
4. Add a thin Python/native episode bridge and an independently owned RL package.
   Observations, normalized actions, rewards, samplers, model and optimizer belong
   there. The bridge exposes physical observations/queries and native stops.
5. Characterize launch exploration before large training allocations. Run a
   modest PPO policy using a pinned existing implementation across three seeds.
   Train on complete procedural missions; any curriculum is a declared sampler
   over complete missions, not a mandatory authored-waypoint ladder.
6. Select frozen checkpoints by the declared development rule, evaluate fresh
   held-out missions, and retain native captures, exact command replay, rich
   reports and common outcome statistics. Diagnose unsuccessful learning honestly.
7. Validate, reconcile workflows and record accepted milestones and limitations.

`pd-core` owns physics/contact/mission truth; `pd-run` owns native episodes;
`pd-missions` owns input generation and cohorts; `pd-eval` owns common evidence;
`pd-report` owns saved projections. Approach adapters own executable dependencies.
The RL package must have no incumbent fallback or planner/controller import.

## Sampling, selection and initial budgets

Declare recipes and cohort seeds before controller outcomes. Keep the original
value-noise family and add a layered regular/ridged/warped/sparse-feature family.
Generator versions describe actual computations; a different noise backend is
not mislabeled as the exact Pylander generator. Preserve the terrain between
locally prepared pads. Structural retries and exhausted requests remain visible.

Initial input characterization is 25,000 scheduled requests per family, repeated
in reverse order. This is input coverage, not solvability or flight evidence.
Use seed-disjoint training, development, validation and held-out cohorts, checking
physical duplicates across them. Training can stream a declared deterministic
proposal schedule; persist consumed indexes and recipe/seed identities.

The recurring development panel has 128 inputs; broad development targets 1,024.
Validation/checkpoint selection rules and final held-out campaign size are fixed
after a small timing-only pilot, before opening held-out results. Final held-out
coverage targets 2,048 missions balanced across both families and directions.
The pilot may reduce this count for a documented resource bound, not a score.
Keep repeated attempts separate from unique input denominators.

Record one initial learning design and at most two experimental revisions. Each
revision states a general hypothesis and is evaluated on development inputs.
Training uses three random seeds and bounded transition/wall-time allocations
chosen from measured throughput. Reserve final evaluation time. Inspect reward
components and actual launch, flight, crash, timeout and landing outcomes;
increasing reward alone is not learning acceptance. Choose checkpoints with a
predeclared development landing-first rule, retaining all seeds and unsuccessful
runs. Freeze the selected design before final held-out evaluation.

The raw incumbent retains unsupported leftward missions in its accounting.
Any reflection adapter is a separately named, globally defined approach with
native verification on original inputs. Never shrink the main denominator to
the incumbent's supported subset. A separately labeled supported-slice comparison
is permitted. Offline solvers retain their own internal guards and proof; do not
force them into a fictitious per-tick policy or alter their mission clocks.

## Authorization and preservation

The loop may implement local code/docs, install pinned ordinary dependencies in
a project-local environment, build, generate new inputs, run bounded training
and evaluation, and commit validated owned milestones. Routine interface,
parameter and backend decisions within these contracts do not require another
confirmation. Continue to the next accepted milestone rather than stopping at
the first integration wrapper. Roughly a day is capacity, not an obligation to
idle or repeat successful checks.

No push, publication, default/accepted-selector promotion, server lifecycle
change, broad cleanup, retained-output deletion, worktree or delegation is
authorized. Preserve the ten starting dirty report/navigation/server paths by
bytes and hunks. Narrow necessary shared-file extensions require explicit
preservation checks. Physics/contact/clocks, current solver behavior, frozen
fixtures, legacy descriptor encodings and historical receipts remain unchanged.

Fresh evidence is create-only under `outputs/procedural-learning/`. Capture
commands must not publish. Summary-mode training retains no command trace and
cannot establish saved-command landing proof. Final landing claims require the
complete physical/mission/integrity/replay tuple; failure, unsupported and
integrity dispositions remain separate.

## Completion and validation

Completion requires two characterized native families, accountable disjoint
cohorts, scalable common comparisons, a working independent training interface,
actual three-seed optimization, frozen checkpoint native evaluation, replay and
rich reports, current workflows and an explained checkout. Improving on the
incumbent is an experimental outcome, not a license for per-case rules.
If an allocation ends with required work unfinished, record it without claiming
the entire goal complete.

Use focused generator/bridge/evidence tests before integrated maintained checks.
The maintained gate is `rtk proxy node scripts/check-planner-development.mjs`.
Run retained 44-case parity only when changing an affected existing execution
path; normal checks never require retained outputs. Generator and environment
checks must run from declared inputs without the old studies or Pylander checkout.
Validate wrapper/core and captured/summary physical equivalence, termination
semantics, normalization/checkpoint identity and tamper rejection. After passed
checks, repeat only for relevant changes or unresolved failures.

## Accepted input and training-interface milestone

The two-family 50,000-request characterization and 4,096-input disjoint cohort
check passed. The thin native bridge and independent Gymnasium package passed
three environment contract tests, including physical equality between captured
and summary execution, exact saved-command replay, and byte-tamper rejection.
The maintained 18-check development gate passed on this milestone.

A 65,536-transition timing pilot, seed 701, completed in 17.25 seconds including
two 128-input development evaluations (about 5.76 seconds each). It was a real
optimizer pilot, not final competence evidence. Both development checks found
zero native summary target successes. Its first development exposure receipts
are explicitly retrospective because collection binding was added afterward.

The initial experiment allocation is now fixed: seeds 801, 802 and 803, each
8,000,000 requested PPO transitions, at most 7,200 wall seconds, with recurring
development every 524,288 transitions. All runs and checkpoints are retained.
Native action recording remains 60 Hz; this approach makes a decision every six
updates and declares the upright first 90 physics steps as its own launch
primitive. These choices are not universal platform task requirements.

For each seed, select its best checkpoint using the declared 128-input
development ordering. Broad development uses all 1,024 inputs; validation uses
512. Those are diagnostic confirmations, not another checkpoint-ranking search.
Freeze all three selected checkpoints before the held-out campaign. Evaluate
each on the same 2,048 held-out inputs with native captures and command replay.
Report each seed separately and distinguish repeated algorithm attempts from
unique mission coverage. The raw incumbent comparison keeps unsupported inputs
in its full denominator; any mirrored adapter is named and scored separately.
