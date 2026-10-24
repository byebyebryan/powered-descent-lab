# Arrival timing and goal selection

This pass follows the [composition results](arrival_composition_execution.md)
and the research review. It separates better landing timing from the value of
an intermediate goal before conditionally fitting a small selector.

## Scope and preserved contracts

Use the frozen flat airborne task, corrected `native_pair_v2` arrival profile,
unchanged tolerances, 0.72 rad tilt bound, native rotation/fuel dynamics and
accepted Poly burn arithmetic. Keep the original 30 s arrival and 90 s mission
deadlines. Switch at the first actual arrival resolution without resetting the
plant, attitude, fuel or clocks. Direct has no intermediate instruction.

The existing first-feasible selector remains the default. Add an explicitly
named opt-in search over exactly the same 7 coast times and 9 burn durations.
Evaluate every admissible pair with the same prepared-coast/native forecast;
rank predicted stable target landings by remaining fuel, with grid order as the
deterministic tie break. A complete captured execution, not its forecast,
establishes landing. Preserve old wire formats, feedback and execution when the
new option is unselected. This is best within a finite bank, not global optimality.

## Frozen questions and phases

1. Check exact preservation against all 180 exposed confirmation executions.
   Compare direct, support and braking under both timing selectors on those
   same 60 originals. Existing confirmation is exposed development evidence.
   Apply improved continuation consistently after every choice's actual handoff.
2. A cost pilot uses originals 000, 015, 030 and 045, all three choices and both
   selectors: 24 Summary executions. Six fixed new-selector capture roles are
   000/direct, 000/support, 015/braking, 030/direct, 045/support and 045/braking.
   Pilot cap: 50 million charged steps and six captures. Pilot observations may
   determine cost allocation, not goals, tolerances or guidance tuning.
3. Before remaining measured work, freeze its exact schedule and reservation
   from pilot costs. Complete the 60-state crossed comparison. Capture all three
   new-selector choices on originals 000/005/010/015/020/025/030/035/040/045/050/055,
   plus each other original's best eligible choice under a predeclared rule.
   Duplicate roles share a capture. Capture every failed new-selector execution.
   Old captures remain linked; new default-profile captures are unnecessary when
   exact result/fingerprint preservation is demonstrated.
4. Release learning only if all new-selector attempts fulfill their instruction
   (when present) and land, hindsight goal choice saves at least 1% mean fuel
   against improved direct, and at least 10% of originals save more than 1 kg.
   Otherwise report a useful negative result and stop before training. These are
   engineering release criteria, not population reliability or significance claims.
5. If released, use independent input-only cohorts from the unchanged goal rule:
   at most 120 training, 36 validation and 60 final-test originals, with distinct
   hash streams and no outcome replacements. Freeze exact sizes, seeds and model
   candidates before generation. Split by original physical state, retaining all
   its choices together; audit physical separation from all earlier cohorts.
   The current 60-state confirmation never becomes fresh again.
6. Fit a small deterministic cost predictor using physical state and offered
   goal contracts. At most 20 closed-form fits; no sequential RL/gradient updates
   are authorized in this pass. Compare always-direct, the best fixed choice
   selected on training, a simple state-based rule and the validation-selected
   predictor. No input IDs, family labels, witness tapes, future outcomes or
   hindsight feasibility masks are policy inputs. Count goal construction,
   forecasts, fitting and deployed inference separately.
7. Validate before opening the final cohort. Release final testing only for a
   selector with no observed instruction/landing failure and at least 0.5% mean
   fuel reduction against improved direct on validation. Once opened, retain all
   final outcomes without tuning. Capture every final choice, with full physical,
   integrity, replay, controller and goal/selection reconstruction evidence.
   Report paired fuel differences and uncertainty over original states, tails,
   contact margins, total search work and deterministic fitting scope. A failure
   is never erased by averaging only successful flights.

## Allocation and stop conditions

Create-only root: `outputs/arrival-timing-selection/loop-20261024T171406Z`.
The baseline binds head `2cabd70`, all 924 Git-visible file hashes and the exact
foreign worktree status. Outer deadline is Unix 1792872846 (three hours).
Limits: 1.2 billion charged native/model/forecast/neutral-replay steps, 256 full
captures, 8 GiB new artifacts, one native worker and a 40 GiB free-space floor.
Build/software tests are separate; measured campaign work and reconstruction
are charged. Reserve proof work before capture; faults retain receipts and stop
the phase. Cost release may reduce the learning cohort before generation if its
complete schedule/proof cannot fit, or stop honestly at the timing result.

Run meaningful preservation, forecast/execution and resource-limit tests, then
the maintained development gate. Use frozen source/library/input ledgers for
each runtime epoch. Independently audit captured predicates, source hashes,
accounting and policy decisions. Commit completed slices locally. New report
delivery is authorized using the established rich tree and detail templates;
retain earlier reports and use their saved previews as appropriate. Do not push,
promote defaults, relax tilt/tolerances or change report-server lifecycle.

## Research basis

- [HIRO](https://arxiv.org/abs/1805.08296): high-level goals and lower-level
  execution; changing the lower behavior changes the higher-level learning task.
- [Constrained subgoals](https://arxiv.org/abs/2006.11485): motivates a limited
  initial goal family, without transferring an optimality theorem to this menu.
- [Contextual selection](https://www.schapire.net/papers/www10.pdf): one choice
  from state/action features. Our all-choice simulator supplies full outcome
  information, so supervised cost prediction is a baseline before sequential RL.
- [Free ignition and constrained descent](https://arxiv.org/abs/1811.10803):
  motivates controlling timing and trajectory comparisons together; it does not
  qualify a different solver or idealized plant as native evidence here.
- [Evaluation uncertainty](https://arxiv.org/abs/2108.13264): report distributions
  and intervals, adapted to paired original-state sampling in this experiment.

The [execution record](arrival_timing_selection_execution.md) owns releases,
outcomes, source epochs, validation, costs and any stop decision.
