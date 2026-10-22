# Planner and controller design review

[Design](maneuver_execution_design.md) · [Contract checker](../studies/maneuver_interface_review/README.md) · [Documentation home](README.md)

October 21, 2026. The reviewed direction is **mission planner plus maneuver
controller**, supported by the native simulation, composition harness and
independent evaluator. Executor responsibilities live inside the controller;
operator identifies a human, deterministic or learned decision source. The first
capability is committed terminal landing on the mission's existing target.

Source review and bounded abstract contract checks support this interface
proposal. A production adapter, native integration, controller qualification and
any learning allocation remain pending. The design's initial three-role split
has been reconciled into two public flight-decision interfaces.

## Research basis

These primary sources motivate individual choices. The project applications
below are design judgments, not evidence that the papers qualify this vehicle,
controller or task. No framework or learning algorithm is adopted by citation.

| Primary source | Relevant result or contract | Application here |
| --- | --- | --- |
| Sutton, Precup and Singh, 1999, [Between MDPs and semi-MDPs](https://people.eecs.berkeley.edu/~russell/classes/cs294/f05/papers/sutton%2Bal-1999.pdf), sections 2 and 3 | Options combine an initiation domain, feedback policy and termination condition. Their execution spans primitive steps; the paper also treats policies depending on execution history. | Declare maneuver admission, ongoing feedback and end conditions. Record actual duration and distinguish planner decisions from internal control updates. Do not assume physical observations erase controller history. |
| ROS 2, [Actions design](https://design.ros2.org/articles/actions.html), interface and goal states | Goal acceptance, execution feedback and final result are distinct. Cancellation can be requested and accepted before completion of cancellation. | Give requests explicit identity, acknowledgement and terminal disposition. The first committed capability rejects cancellation; future cancellation needs defined physical continuation. Use these semantics locally without adding ROS middleware. |
| Nachum et al., 2018, [Data-Efficient Hierarchical Reinforcement Learning](https://proceedings.neurips.cc/paper_files/paper/2018/file/e6384711491713d29bc63fc5eeb5ba4f-Paper.pdf), introduction and section 3 | Changing lower-level behavior changes the meaning of actions for the higher-level learner; the paper develops a correction for its learned hierarchy. | Freeze controller configuration during an initial planner comparison. A later backend change needs new compatibility/performance evidence; this design does not implement or require that paper's correction. |
| Agarwal et al., 2021, [Deep Reinforcement Learning at the Edge of the Statistical Precipice](https://proceedings.neurips.cc/paper/2021/file/f514cec81cb148559cf475e7426eed5e-Paper.pdf), sections 3 and 4 | Few independent training runs can yield misleading point estimates; evaluation should represent uncertainty and variability. | Keep independent fits separate from mission counts. Plan paired/clustered accounting for shared missions and mirrors, report per-fit outcomes, and choose uncertainty estimates for the actual sampling design. A fixed regression corpus has a different interpretation from fresh population sampling. |

## Source review and decisions

| Finding | Evidence | Design disposition |
| --- | --- | --- |
| Existing terminal guidance already generates commands directly | [Terminal update](../pd-control/src/terminal/mod.rs) and [Controller trait](../pd-control/src/lib.rs) | Wrap it with a maneuver lifecycle; keep ZEM/ZEV, candidate selection and touchdown logic inside one backend |
| Entry readiness is advisory and can select urgent best effort | Terminal entry assessment and [state DTOs](../pd-control/src/terminal/state.rs) | Separate supported-request admission from predicted landing feasibility; the initial planner has no backend-specific advice input |
| Backend behavior depends on retained state | Terminal hysteresis and optional retained arrival plan | Preserve backend state on Keep, duplicate and rejected requests; do not promise a generic Markov planner input before future goal revision/composition |
| Command semantics include minimum throttle and attitude actuation at engine-off | [Native plant](../pd-core/src/sim.rs) | Preserve raw engine-setting/absolute-attitude semantics and declare upright coast as assistance |
| Targets are mission-bound and native goals can end on waypoints | [Run context and goals](../pd-core/src/model.rs) | Scope initial support to airborne route-free LandingOnPad missions and their existing target; future retargeting is explicit |
| Generic Agent execution has no terminal-result callback | [Agent loop](../pd-run/src/agent.rs) exits at stop; [Episode step](../pd-run/src/episode.rs) exposes actual end | Composition explicitly closes the maneuver from a returned native execution/step result; no command callback after native end |
| Physical replay does not reconstruct the decision logic | [Native evaluator](../pd-eval/src/episode.rs) replays saved commands independently | Retain controller-request and planner-decision reconstruction as additional checks, with a separately bound structured trace |
| Constructor configuration omits live plan state | [Resolved configuration](../pd-control/src/terminal/config.rs) and terminal constructor | Descriptor identity is not a resumable checkpoint; durable controller state or exact original-prefix reconstruction remains an implementation gate |
| Existing retained-arrival evidence has a specific envelope and command clamp | [Recorded workspace sources](maneuver_execution_design.md#controller-alternatives-and-comparison-identity) and original isolated-worktree receipts | Keep ordinary terminal defaults as the parity reference; bind the complete candidate configuration before comparing retention |

The initial request vocabulary now distinguishes Keep, a request identity and
LandOnMissionTarget. A new goal while landing is Busy; reuse of the same ID and
payload is idempotent; a changed payload under that ID is invalid. Semantic
rejections preserve the existing owner and consume normal physical time. Native
end always precedes another planner or controller callback.

The earlier fixed-interval training sketch did not fully distinguish available
decisions from compulsory continuation after landing activation. The revised
design keeps a 10 Hz decision grid before commitment and a 60 Hz controller
throughout execution. Its preferred future learning binding groups all actual
native execution until the next choice or episode end into one decision
transition, retaining duration and time-correct reward/discount accounting.
A fixed-interval alternative must label forced continuation and establish
equivalent accounting before use. Neither choice supplies a reward preference
or claims that a learner is ready.

## Bounded validation completed

The [offline checker](../studies/maneuver_interface_review/check_contract.py)
uses an abstract lifecycle and a changing command token, not vehicle dynamics or
the actual terminal implementation. It explores reachable model states and
checks invariants independently of its admission rules. Deliberate faults must
produce counterexamples; an undetected fault fails the checker.

| Check | Result |
| --- | --- |
| Lifecycle exploration through depth 8 | 209 reachable abstract states and 1,458 outgoing event checks; passed |
| Explicit admission outcomes | 8 cases; passed |
| Independent 120/60/10 Hz clock examples | 128 schedules, including odd-physics-tick end and zero-length episode; passed |
| Return aggregation | 1,024 exact-rational examples over every partition of eight base steps, unit/non-unit discount and terminal/nonterminal bootstrap examples; passed |
| Lifecycle negative controls | All 7 deliberate faults detected with retained counterexamples |
| Return negative controls | Both omitted intermediate rewards and incorrect duration discount detected |
| Native flight or optimizer work | Zero |

The lifecycle faults are duplicate reset, loss of active ownership on rejection,
resurrection after native end, clock reset on activation, hidden entry-gate wait,
stale held backend commands and invented completion. For example, the model
rejects `start_a → start_a` when it resets the backend, and `start_a → keep`
when it holds the old command instead of updating the active backend.

The [retained receipt](../outputs/design-review/planner-controller-20261022T004344583015Z/contract-validation.json)
binds checker SHA256
`f07bf9817ef01de94a8200c2f581907f7bc13523082c226e7746d48ac9b60cef`.
The UTC receipt directory falls on October 22; the design/review date is October
21 in the workspace timezone. Reproduce the checks from a fresh checkout with:

```sh
rtk proxy python3 -B studies/maneuver_interface_review/check_contract.py
```

State merging makes the bounded exploration finite; it does not enumerate every
redundant event history or prove unbounded behavior. The checks establish model
consistency and accounting identities, not native timing, restart equality,
trace-binding integrity, landing capability or statistical power. These remain
separate acceptance steps.

Documentation validation passes the local link/anchor check, all 13 relevant
Node tests and `git diff --check`. The native planner/controller implementation,
frozen inputs and pre-existing evidence remain unchanged by this review.

## Acceptance before flight and learning

1. **Native adapter integration.** Implement the controller facade and existing
   terminal backend adapter. Check actual request/reset/duplicate/error behavior,
   immediate command authority and final-result delivery with the real runtime.
   Preserve existing IDs, defaults, physical semantics and serialized contracts.
2. **Composition and continuation.** Implement deterministic planners and the
   one-episode harness. Validate real 60/10 Hz scheduling, partial native ends,
   decision-opportunity flags, exact continuation and create-only trace binding.
   Verify source/descriptor identity and reconstruct requests and commands.
3. **Terminal numerical parity.** Under identical backend configuration, compare
   direct terminal execution and immediate activation across the maintained
   clean/trajectory-error packs. Require exact underlying commands, states,
   clocks, fuel, contact and outcomes, retaining failures. Assess additive
   request telemetry/identity separately; equal landing totals are insufficient.
4. **Bounded characterization.** Measure cost first, freeze inputs/schedule and
   limits, then compare activation choices and the selected controller variant.
   Keep observed regressions, fresh qualification, frontier/stress inputs and
   controller failures separate. Preserve reproducible original mission prefixes.
5. **Learning release.** Require a measured usable controller, a meaningful
   remaining planner decision, declared return/time semantics and independent
   fit/selection/final-evaluation rules. A deterministic solution can close the
   initial timing task without a learning allocation.

Controller tuning, general arrival-state control, goal cancellation/preemption,
new launch/correction capability and broad terrain learning require their own
scopes and evidence. The review does not expand the accepted planner or reopen
an old learning allocation.
