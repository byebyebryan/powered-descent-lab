# Coast ignition stopping policy research and plan

[Documentation home](README.md) · [Original experiment design](rl_coast_ignite_plan.md) · [Completed hybrid result](rl_coast_ignite_execution.md) · [Poly Landing contract](poly_landing_controller_plan.md)

October 22, 2026. **Proposed implementation and experiment pass.** First compare
stopping rules using the existing frozen policies and unchanged Poly Landing.
Add admission and contact diagnostics so the comparison can explain its failures.
Further training and a stronger controller admission contract depend on those
results. This plan allocates no native campaign, training, report regeneration,
commit, push or publication by itself.

The owner subsequently authorized the [execution loop](rl_coast_ignite_stopping_execution.md),
which owns the frozen allocation, source epochs and actual phase results.

## Evidence and the question to answer

The completed hybrid lands 589/600 fresh supplied arcs, versus 446/600 for the
validation-selected three-second delay. Those inputs and all eleven misses are
now exposed evidence. Preserve the result, captures, source epochs and HTML
report; use new inputs for subsequent selection and confirmation.

Read-only investigation checked capture and decision hashes for all 44 matched
attempts underlying the eleven RL misses. Algebraic reconstruction from saved
activation observations reproduced all eleven admission decisions and actuation
scan counts. For accepted requests, reconstructed coefficients matched saved
coefficients within 1e-12. No new native transition was used in that diagnosis.

| Miss group | Fresh arc indices | Evidence |
| --- | --- | --- |
| Above maximum thrust at ignition | 285, 355, 503, 546, 568 | First paired inverse exceeds maximum authority; no burn starts |
| Below minimum thrust later in the proposed burn | 269, 491 | Complete admission scan stops at local ticks 798 and 994 respectively; no burn starts |
| Accepted reference contacts too early | 64, 436, 461, 514 | Actual position stays within 0.3 mm of the exact reference up to contact; the reference dips below usable landing height before returning toward its endpoint |

The four accepted burns contact at 6.233, 9.000, 7.642 and 8.383 seconds, while
their scheduled endpoints are 12.467, 14.933, 12.267 and 12.517 seconds. Their
minimum reference center heights are approximately -1.391, 2.174, 3.169 and
4.285 m. The upright touchdown center convention is 4.995 m.

Case 514 meets the individual speed and attitude limits at contact, but its
feet have clearances -0.0123 m and 0.3719 m. The native stable-contact condition
requires maximum foot clearance at most 0.15 m. This explains the crash without
changing the physical verdict or relaxing contact geometry. The other three
also exceed the normal-speed limit; case 64 exceeds tangential speed as well.

Every miss ignites 1.3–1.9 seconds after its generator witness time. Successful
arcs include delays as large as 2.5 seconds, so this observation does not justify
a universal offset. Witness timing remains diagnostic provenance, excluded
from features, rewards, labels, masks and decoder decisions.

The evidence root is
`outputs/coast-ignite/loop-20261022T173645Z/campaign-c/learning/`.
Its `selection-freeze.json`, `confirmation/results.jsonl`, per-case
`evaluation.json`, `decisions.json`, `seal.json` and `proof.json` bind the result.
The returned admission reason was discarded by the example bridge;
the seven reasons above are reconstructed arithmetic diagnoses, not saved reason
strings. The [complete HTML tree](http://starship:8000/coast-ignite/loop-20261022T173645Z/campaign-c/learning/report-final-v2/report.html)
retains all outcomes and rich details.

## Research conclusions

This remains a finite-horizon optimal-stopping problem. After commitment, the
payoff is the eventual native outcome of the fixed burn. For Markov stopping
problems, deterministic stopping decisions can suffice; randomness is not a
requirement for a good final policy. Becker, Cheridito and Jentzen also turn
their specifically trained soft stopping functions into hard decisions at
one half. Their construction does not establish an extraction guarantee for
our PPO hazard residual. See [Deep Optimal Stopping, sections 2.1–2.3](https://jmlr.org/papers/volume20/18-232/18-232.pdf).

SB3's categorical deterministic action is the local probability argmax, while
sampling draws from the categorical distribution. Its documentation explicitly
recommends trying deterministic PPO evaluation and notes that it frequently
improves performance. The existing use of greedy evaluation is therefore a
reasonable choice to test, not an identified implementation bug. See the
[pinned distribution implementation](https://stable-baselines3.readthedocs.io/en/v2.9.0/_modules/stable_baselines3/common/distributions.html#CategoricalDistribution)
and [evaluation guidance](https://stable-baselines3.readthedocs.io/en/v2.9.0/guide/rl_tips.html#how-to-evaluate-an-rl-algorithm).

Our frozen model's probability is a conditional ignition hazard. Along each
recorded coast prefix, the probability that training-time sampling would reach
the greedy ignition tick without an earlier request is only 4.36–7.13% for the
eleven misses. Offline inference reproduced every recorded decision in those
prefixes. This shows a material difference in stopping-time distributions; it
does not establish that the earlier sampled requests would land. Earlier
ignition can also be invalid. That causal question needs matched native runs.

Trajectory feasibility includes state constraints as well as control bounds.
The distinction is explicit in the trajectory-generation formulation reviewed
by [Malyuta et al.](https://arxiv.org/abs/2106.09125). Here, the controller source
is the decisive evidence: checked admission scans thrust and fuel but does not
check intermediate body clearance or safe first contact. Accurate execution
cannot repair an unsafe reference. This supports clearer diagnostics and a
separate future admission design, without requiring a new trajectory solver.

Evaluation must preserve uncertainty across inputs and learned packages.
[Agarwal et al., 2021](https://papers.nips.cc/paper/2021/hash/f514cec81cb148559cf475e7426eed5e-Abstract.html)
motivate interval estimates and explicit variability across runs. Repeated
action samples on one arc do not create independent missions, and three
previously selected checkpoints do not establish broad training-seed robustness.

## Stopping rules to compare

Let h_k be the frozen network's ignition probability at opportunity k, conditional
on having issued WAIT previously. The physical coast prefix is common until a
request or native contact. Define survival and cumulative stopping probability:

```text
S_before(0) = 1
P(request at k) = S_before(k) * h_k
S_after(k) = S_before(k) * (1 - h_k)
F(k) = 1 - S_after(k)
S_before(k + 1) = S_after(k)
```

| Rule | Request condition | Purpose |
| --- | --- | --- |
| G Current greedy | h_k > 0.5; ties WAIT | Preserve the deployed comparison exactly |
| S Seeded sampling | u_k < h_k, using a precommitted per-arc random stream | Evaluate the learned stochastic policy |
| M Cumulative median | First opportunity with F(k) >= 0.5 | Test one fixed deterministic summary of the induced stopping distribution |

G must call the original `model.predict(..., deterministic=True)` path, preserving
its floating-point evaluation and tie behavior. The table states the mathematical
rule, not permission to replace the original inference with a new approximation.

M is an engineering comparator, not an optimality claim or a recommended
replacement in advance. Do not sweep quantiles or local probability thresholds
in this pass. M uses only observed past hazards and one planner-owned survival
scalar; it requires no future rollout or controller feasibility feature.

Use log survival with defined zero/one-hazard behavior for numerical stability.
At the final opportunity, a rule that has not requested ignition issues WAIT
and commits to never ignite. Preserve the residual never-ignite mass. Native
contact censors later decisions; do not renormalize the distribution conditional
on eventually igniting or on surviving contact.

For the unchanged uniform prior h_k = 1/(82-k), all 81 ignition opportunities
and never-ignite have mass 1/82. G never ignites because its largest hazard is
0.5, while M chooses slot 40, at 4.0 seconds. This arithmetic distinguishes the
rules; it is not a landing or learnability result.

S must be reproducible independently of process order, vector-environment order
and unrelated RNG use. Freeze a domain-separated SHA-256-to-uniform construction
over a declared randomization key, physical arc hash, replicate index and tick.
Specify canonical encoding and integer-to-float conversion with golden vectors.
Use the same uniforms across frozen weights on matched arcs. Record each draw,
hazard and decision; no outcome-dependent seed selection or retry is permitted.
These identifiers select randomness only and are absent from network inputs.

## Planner and controller responsibilities

Retain the ten physical/context features, 10 Hz decision clock, original native
episode, 60 Hz commands, 120 Hz physics, initial-attitude hold, fixed absolute
reference end and one ignition attempt. The planner owns the stopping rule;
the controller owns every throttle and attitude command. IGNITE begins the
powered burn immediately at that native boundary. NoPlan retains engine-off
execution to the actual stop. No automatic retry, earlier forced ignition,
duration substitution, reward shaping or mid-burn correction is introduced.

The public admission status means that this backend constructed an executable
finite reference within its declared thrust/fuel checks. It is not a touchdown
certificate. Keep that distinction visible in new diagnostics. A stronger path
admission rule would need its own versioned contract and comparison; rejecting
these four accepted requests alone leaves their matched passive-coast crashes.

Add versioned example-level sidecars containing the returned AdmissionV2 value,
activation observation and clocks, first failing actuation pair where available,
per-decision hazard and rule state, and native incoming-contact margins. Recover
diagnostic reference coefficients from the existing builder. Reuse core contact
geometry rather than creating a second physical verdict implementation.
If richer foot geometry needs new access, expose an additive pure projection
of the existing core snapshot with parity tests; preserve existing DTOs and
the classifier, thresholds and integration unchanged.

Reference-only diagnostics should show the first predicted body contact,
its state and requested attitude, center-height extrema and endpoint residual.
Label assumed instantaneous pointing explicitly. Do not call that geometry an
attitude-servo forecast or native certification. A blanket negative-clearance
guard is also incorrect: the established terminal center intentionally
undershoots by 0.005 m to obtain native contact. Keep diagnostics out of action
selection, reward and admission during this experiment.

## Proposed sequence and allocation

Freeze the three checkpoints already selected by the completed experiment:
281101 at 65,536 decisions, 281102 at 131,072, and 281103 at 98,304. Preserve
their source/model hashes. Seed 281103 remains the primary package throughout;
do not reselect its weights or training seed. This pass has zero optimizer steps.

Use the original input generator and all original input-only admission rules.
New development, validation and confirmation streams must be mutually disjoint
and exclude physical identities from all retained training, validation and fresh
inputs. Freeze stream keys, ordering and the complete raw-proposal allowance
before release. Preserve exclusions and underfill; never screen with a native
witness flight, timing sweep or policy outcome.

| Phase | Allocation and evidence | Advance condition |
| --- | --- | --- |
| A Software and diagnostics | Golden tests for decoders and randomness; read-only reconstruction of the eleven misses; 16 captured G preservation cases, comprising those eleven plus successful arc indices 0–4 | Same commands, clocks, fuel, native outcomes and physical replay; added sidecars do not change accepted arithmetic |
| B Development | 128 new arcs; each of three weights under G once, M once and S eight times; fixed three-second delay and immediate ignition once per arc: 4,096 episodes | Complete declared inventory, finite inference, reproducible decisions and no software fault; no rule or model changes after this phase |
| C Validation | 256 new arcs under the same schedule: 8,192 episodes | Select one common rule under the prospective rule below; freeze its full package before confirmation |
| D Conditional confirmation | 1,200 new arcs under selected rule for each of three weights, original primary G and fixed three-second delay: 6,000 captured episodes; if S wins, three additional primary-package draws on the first 128 arcs: 384 episodes | Complete physical/mission/integrity/replay and independent controller plus live planner reconstruction for every occurrence |
| E Delivery | A separate established HTML mission tree with matched attempts, rich details, stopping traces and failure diagnostics | Full inventory, browser checks and exact served bytes if publication is authorized; original report unchanged |

The first four development arcs form the cost pilot within the 4,096 total,
not another allocation. Capture those 128 occurrences for cost/proof estimates.
Development and validation otherwise use Summary outcomes for selection, with
all decisions retained; these are not advertised as replay-certified landings.
Capture and verify the selected rule and primary G on the first eight validation
arcs before confirmation release, adding at most 16 proof-bearing executions.

Validation score is native success averaged equally across arcs and the three
frozen weights. For S, average its eight draws within each arc/weight first.
Choose highest score, with exact ties preferring G, then M, then S. Release fresh
confirmation only if a non-G winner improves both the three-weight average and
the fixed primary weight over their matched G scores by at least 0.5 percentage
points. This is an engineering nomination threshold, not a significance claim.
Otherwise close with the stopping-rule hypothesis unconfirmed and retain the
current package. Do not add quantiles, change weights or restart training.

Fresh primary analysis uses exactly one predesignated occurrence per arc:
replicate zero for S. Compare the fixed primary weight under the selected rule
against primary G on the same 1,200 arcs. The extra S draws are secondary
within-arc variability diagnostics, never 384 additional independent missions.
The other two frozen weights receive separate package results; without matched
fresh G for those weights, do not claim an individual decoder-effect estimate.

## Acceptance and statistical limits

Primary improvement requires positive paired lift over primary G and a
one-sided exact paired discordance test below 0.05. The comparison and direction
are frozen before fresh release; neither the old 600 arcs nor the heuristic
comparison enters that test. Report gained and lost landings, both package
rates with exact intervals, and paired lift with a paired interval. The heuristic
is contextual and is not reselected. No fresh stopping or retuning is allowed
based on partial performance; ordinary misses count through the whole schedule.
Use a paired percentile bootstrap with 20,000 resamples of whole arc pairs for
the lift interval, with its RNG key fixed before fresh release. Report that
interval alongside the exact paired test and exact marginal intervals.

Separately report whether the selected primary package's one-sided 95% exact
lower bound reaches 99% for this same supplied-arc task. At 1,200 independent
primary occurrences that requires at least 1,194 successes. This is a proposed
new reliability criterion; the old study's advantage claim remains unchanged.
An improvement without that bound is still an improvement, with reliability
unqualified. A failed paired test may be inconclusive with few discordances;
1,200 arcs do not promise power to resolve every sub-percentage-point gain.

Report phase, direction, reference duration and initial-energy breakouts as
descriptive subsets, with denominators. Include all three weight results and
all sampled replicates without choosing a best fresh seed. Fresh physical
proof, RNG reconstruction and the same complete native success tuple are
required even when Summary results look promising.

Proposed outer caps are one native worker, two hours, 200 million total native
steps including every replay/reconstruction, 20,000 original executions and
48 GiB of new artifacts, with at least 80 GiB free before starting. Original
executions total at most 18,704 including the specified proof captures and
preservation checks. The 128-episode cost pilot has separate caps of five
minutes, one million original native steps and 1 GiB. Its observations freeze
later phase reservations within the outer caps before those phases start.
If rich captures, proof or delivery cannot fit, stop before fresh release.
Do not shrink the fresh denominator, omit proof, prune retained evidence or
substitute a simplified report to fit a cap. An execution declaration must
record exact counters, source epochs, interruption and create-only output rules.

## Implementation slices and review checks

| Slice | Scoped responsibility | Validation |
| --- | --- | --- |
| Diagnostic bridge | `examples/coast-ignite-policy/src/lib.rs`, experiment sidecars and an additive pure core geometry projection if needed | Capture the returned admission; preserve the controller and old DTOs; prove unchanged physical records on the 16 declared cases |
| Stopping rules | `experiments/coast-ignite/coast_ignite.py` or a small adjacent module | G bit-for-bit decisions; S golden randomness, scheduling independence and empirical categorical frequencies; M zero/one hazards, exact threshold, never mass, native censoring and episode reset |
| Runner and proofs | Opt-in experiment runner adjacent to `run_loop.py` | Fixed hashes/streams, zero optimizer calls, exact inventories, separate repeat/arc denominators, resource counters and live decoder reconstruction |
| Reporting | Existing experiment renderer and maintained rich templates | All attempts reachable; admission reasons, hazard/survival and first-contact geometry visible; original template/navigation preserved |

Do not extend neutral pd-python or Policy 3 to host this experiment. Keep
reference/actuation math, physical integration, native contact classification,
frozen fixtures and current default agents unchanged. No new flight behavior
belongs in a documentation or diagnostic cleanup slice.

For implementation, run focused Rust/Python checks plus the maintained
`rtk proxy node scripts/check-planner-development.mjs` gate. Numerical parity
against retained captures remains explicit opt-in. This research document needs
the docs-link checker, its Node tests and `git diff --check`; no native campaign
or report operation is required for the planning pass.

## Decisions after this pass

If a frozen-weight decoder improves independent confirmation, retain that
package and consider a separate training study that evaluates the intended
deployed stopping rule throughout checkpoint selection. If G remains strongest,
keep it and investigate learning coverage or direct stop-versus-continue value
learning under a new bounded proposal. Neither branch authorizes an algorithm
sweep or more PPO steps against the exposed misses.

A stronger controller admission contract remains a separate design question:
what can be certified from current state, reference, actual attitude authority
and native first-contact geometry without a hidden rescue? It may later reject
unsafe intermediate references, but it must preserve immediate activation and
planner-owned timing. General arrival control, learned duration, departure,
terrain avoidance and fuel optimization remain outside this stopping-rule pass.

The subsequent [timing and efficiency plan](rl_coast_ignite_efficiency_plan.md)
uses the completed result to propose a separate fixed-end handoff map before
allocating fuel learning or duration selection. It preserves this pass's
objective, selection rules and historical outcomes.
