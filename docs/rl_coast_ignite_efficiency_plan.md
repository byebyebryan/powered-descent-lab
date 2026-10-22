# Coast ignition timing and efficiency plan

[Documentation home](README.md) · [Completed stopping-rule study](rl_coast_ignite_stopping_execution.md) · [Original task and reward](rl_coast_ignite_plan.md#reward-exploration-and-learner) · [Poly Landing contract](poly_landing_controller_plan.md)

October 22, 2026. **Proposed characterization pass.** Map successful handoff
times and their fuel costs under unchanged Poly Landing before choosing another
learning objective. The first pass holds each supplied reference-end time fixed.
Joint ignition/end-time selection is a possible later experiment. This plan
launches no implementation, native campaign, training, report refresh, commit,
push or default promotion.

The owner subsequently authorized the [execution loop](rl_coast_ignite_efficiency_execution.md),
which owns the exact allocation, source epochs and measured results.

## What the retained evidence establishes

The completed frozen-weight study lands 1,193/1,200 fresh arcs with cumulative
median stopping, versus 1,176/1,200 with the original greedy rule. The paired
improvement passes; the separate 99% reliability criterion remains unmet.
Controller arithmetic is unchanged and the study performs zero optimizer steps.

A subsequent read-only reduction compares the 1,172 primary arc pairs where
both packages have verified target landings. Fuel consumed is scenario initial
fuel minus native final fuel. Coast preserves fuel, which was checked against
each recorded activation. All paired physical identities match the saved input.

| Mean over the same 1,172 arcs | Greedy G | Cumulative median M |
| --- | --- | --- |
| Ignition after episode start | 3.711860 s | 3.151195 s |
| Activation to actual touchdown | 13.782523 s | 14.348372 s |
| Episode start to actual touchdown | 17.494383 s | 17.499566 s |
| Fuel consumed | 409.783084 kg | 409.722234 kg |

M uses 0.060850 kg less fuel on average, a 0.01485% reduction in the ratio of
total consumption. Its median paired saving is 0.117963 kg. It uses less on
1,104 pairs and more on 68. These are post-hoc descriptive comparisons,
conditional on shared success; they omit the 28 arcs where either package
misses and establish no new efficiency qualification or optimal timing rule.

Two native-valid early landings explain the largest opposite fuel effects:

| Fresh arc | Reference end | G ignition / contact | M ignition / contact | G / M fuel consumed |
| --- | --- | --- | --- | --- |
| 488 | 10.233333 s | 1.8 / 6.933333 s | 1.2 / 10.233333 s | 174.476218 / 239.309335 kg |
| 552 | 17.850000 s | 2.8 / 13.216667 s | 2.2 / 17.850000 s | 305.488828 / 393.520541 kg |

Both G flights remain successful by the authoritative contact/mission rules.
Their maximum foot clearances at contact are 0.135994 m and 0.145645 m,
respectively, against the native stable-contact limit of 0.15 m. These are
different terminal geometries from the near-upright endpoint contacts in M.
Keep the successes and costs; expose their contact margins rather than
retrospectively imposing an endpoint or robustness requirement.

As a descriptive sensitivity check, 1,146 pairs contact within two physics ticks
of their respective common reference end in both packages. Their mean M fuel
saving is 0.175694 kg, or 0.04272% of total G consumption. This selected subset
is not a revised primary denominator. The two-tick tolerance is one 60 Hz command
interval, not a new success threshold. Neither summary demonstrates substantial
fuel headroom or rules it out at other ignition times.

Evidence root: `outputs/coast-ignite-stopping/loop-20261022T204723Z/`.
The reduction reads `campaign/fresh/results.jsonl` and its original input files;
the [established HTML tree](http://starship:8000/coast-ignite-stopping/loop-20261022T204723Z/campaign/report/result-v2.html)
retains every matched flight. The saved `independent-audit.json` and
`closure.json` own the original proof and completion claims. At this review the
fresh ledger SHA-256 is
`da9589c474f3d9d3f3cd579d5e781e8ba1b58c491d5a6e8b0b101d8de1361ba0`.
No native transition, optimizer step or report operation was used in this
post-hoc review.

## Timing variables and controller ownership

Use distinct names for episode start, ignition, requested reference end and
actual first contact. For the unchanged first task:

```text
coast duration          = ignition time - episode start
planned burn duration   = supplied reference end - ignition time
actual powered duration = first contact time - ignition time, if admitted
actual flight duration  = first contact time - episode start
contact-time residual  = first contact time - supplied reference end
```

The supplied time constrains reference construction; native safe contact may
end the episode earlier. It is not an enforced exact-touchdown mission deadline.
The 40-second hard deadline and reference-end-plus-four-tick command coverage
are separate boundaries. Actual powered duration above applies to the current
continuous-burn accepted backend; rejected and censored requests have no powered
duration. For an episode without contact, report actual stop and cause separately.

Holding the reference end fixed couples coast time and planned burn duration.
Changing both ignition and reference end would add a second decision dimension
and change the task seen by the frozen network. It would not require replacing
the controller, but needs a separate request/context and evaluation contract.

Planner ownership remains WAIT/IGNITE and, in a future version if selected,
the explicit reference-end instruction. The controller owns admission and every
physical command. Activation starts the burn immediately; no internal coast,
duration search, retry, rescue or mid-burn policy action is introduced.

## Research implications

NASA's [path-adaptive lunar guidance comparison](https://ntrs.nasa.gov/api/citations/20230009339/downloads/M20-7880_Paper.pdf),
especially its time-to-go and ignition sections, treats burn duration and
ignition logic as distinct choices. It also distinguishes minimum control effort
from minimum propellant and compares guidance with optimized trajectories.
This supports measuring actual fuel rather than using burn length or smooth
acceleration as a fuel proxy. Its two-stage lunar results do not predict the
efficiency of this lab's single-stage Earth-gravity controller.

[Szmuk, Reynolds and Acikmese](https://arxiv.org/pdf/1811.10803), sections II.B
and II.D, separately define coast and powered durations and allow ignition state
to vary along a passive trajectory. That supports a possible later joint-timing
formulation. It does not establish that Poly Landing is fuel-optimal or justify
adding a new optimizer now. First measure the existing backend's discrete
handoff landscape, including state/contact constraints as well as actuation.

## Questions for the first pass

1. Which legal ignition opportunities produce native successful outcomes, and
   are they one wide interval, several disconnected intervals or a narrow edge?
2. Where do the frozen M and G choices lie relative to those sets? Can the six
   rejected M requests and its accepted crash be avoided by timing alone?
3. How much fuel can be saved among successful opportunities under the same
   reference-end instruction? Is that headroom broad or confined to early
   contacts near native limits?
4. Does the fuel-efficient choice have successful adjacent 10 Hz opportunities,
   or does a one-slot change lose the landing?
5. Is fixed-end efficiency large enough to motivate learning, or does useful
   improvement require a later end-time/duration study?

## Cohorts and complete discrete scan

Use the retained 128-arc development population as the main characterization
cohort. Include every arc from the declared input inventory, without choosing
cases by policy outcome. They are already exposed development inputs, not a
new fresh test or an estimate of general mission reliability.

Use a separate 30-arc diagnostic pack from the exposed 1,200-arc confirmation:
the union of the 24 G misses and seven M misses contains 28 distinct arcs;
add fuel-outlier arcs 488 and 552. This includes all four M regressions, all
21 gains and the three shared misses. Keep this outcome-selected pack separate
from main-cohort rates and cost distributions. Bind its exact physical hashes
and source paths before executing it; arc indices alone are not identities.

For each of the 158 inputs, test all 81 legal requests at 0.0, 0.1, ..., 8.0
seconds, plus the declared never-ignite schedule: **12,956 Summary executions**.
The first four main-cohort arcs are the cost pilot and are part of these counts.
Do not add sub-grid searches: the policy acts at 10 Hz, so the full discrete
domain can be enumerated without assuming unimodality or monotonic feasibility.

Every occurrence resets the original episode at its original initial state,
coasts through native commands to its requested slot, and runs the same
immediate checked burn or rejected engine-off continuation to the real stop.
There is no teleport to an analytically propagated activation state. If contact
censors the request, record the requested slot and absent actual ignition.
Do not merge those requested schedules or count them as independent missions.
Keep all 82 occurrences, including censored requests, rejections and never.
Separate successful admitted burns from any native-valid passive contact before
a request. A censored request is not a demonstrated successful powered handoff.

Overlay the original frozen primary M/G choices from retained ledgers. The
grid probe scheduler is a diagnostic approach, not a learned planner or an
online hindsight lookup. The map and generator witness stay out of network
features, masks, rewards, weight selection and deployed action logic.

## Measurements and interpretation

Each cell records immutable input/request identities, requested and actual
ignition, admission/reason, stop/cause, native task outcome, physical fingerprint,
initial/final fuel, actual contact tick, reference end, controller coverage and
native contact geometry. Source, binary and schedule hashes bind every record.
Fuel used includes the complete original episode. Failure fuel remains visible
but earns no efficiency credit. If a passive schedule lands successfully, retain
that valid outcome and zero-burn cost rather than assuming never must fail.

For each arc, report the following without collapsing failure and cost:

- Every contiguous run of native-successful grid points, in ticks and seconds;
  distinguish admitted powered handoffs from passive outcomes. Unresolved
  timing between grid points is not a continuous safety certificate.
- Whether M/G land, and whether another tested opportunity succeeds when they
  fail. No successful tested point means bounded absence, not impossibility.
- The least-fuel successful tested point, breaking exact cost ties by earliest
  requested slot. This is the best observed point for this controller, task
  and grid, not a global trajectory optimum.
- Fuel regret of a successful policy choice relative to that point, in kg and
  percent of positive best-point consumption. Failed policy choices have
  undefined fuel regret and retain their failure count.
- The successful points within 1% of best fuel, their component widths and
  the nearest observed failure on either side. The 1% value is a descriptive
  engineering screen, not a new qualification threshold or reward coefficient.
- Neighboring-slot success at the best point and at policy choices. Require
  both adjacent legal slots for a two-sided interior label; otherwise show the
  missing or failing neighbor. This is timing sensitivity, not robustness to
  state, actuator, sensor or physics perturbations.
- Fuel versus actual flight duration and native contact margins. Keep early
  native-valid contact visible. An optional within-two-tick endpoint subset is
  descriptive only and never replaces the complete outcome denominator.

Use per-arc summaries and distributions over the 128 main inputs. Show means,
medians, tails and absolute kg alongside percentages. The 82 schedules on one
arc are correlated alternatives, not 82 independent trials. The hindsight
successful-grid union is a diagnostic ceiling; it cannot qualify an online
policy. No new reliability claim, fresh p-value or model selection belongs here.

Summary outcomes are provisional native observations. Verify the selected best
point on each arc with a full capture before treating its measured landing/cost
as proof-backed. Require exact agreement with the original Summary fingerprint,
clocks, fuel, admission and stop. A mismatch is a software fault that stops the
phase; do not try the next candidate to obtain a passing result.

## Implementation and execution sequence

1. **Read-only reduction.** Add a reusable analysis entrypoint for paired saved
   ledgers. Bind hashes; reproduce the 1,172-pair figures, both outliers and
   optional endpoint subset. Preserve original ledgers/reports and label
   exposed confirmation inputs as diagnostic use. This does not change the
   frozen stopping study's verdict or objective.
2. **Probe runner and preservation.** Add an example-level fixed-slot runner
   using the existing native CoastEpisode interface, source bindings, budget
   counters, create-only schedules and capture/replay facilities. On the first
   four main arcs, fully capture frozen G and M as eight preservation controls;
   reconstruct each live frozen policy and require original ledger equality.
   No controller or plant arithmetic change is needed.
3. **Measured pilot.** Execute the first four complete maps: 328 Summary
   occurrences. Fully capture their fixed 0 s, 3 s, 6 s and never schedules
   as 16 additional cost/proof controls. Preserve completed work as part of the
   final allocation. Measure runtime, pure-query cost, native steps, capture
   bytes and actual rich-report bytes before releasing the remaining budget.
4. **Complete maps and proof.** Finish all 158 maps. Capture up to 158 best
   successful points, up to 60 M/G choices on the 30 diagnostic arcs, and up to
   14 neighbors of best points on the seven original M misses. Those neighbors
   are the immediately preceding/following legal slots, with absent points
   recorded rather than replaced. Reuse a captured occurrence for multiple
   roles instead of creating duplicates; unused capture slots stay unused.
   Grid probes reconstruct their fixed scheduler; learned controls reconstruct
   the real frozen model and decoder, with matching original timing markers.
5. **Review and decision.** Deliver complete Summary maps and verified rich
   examples, audit resource/provenance counts, and choose a specific next
   hypothesis. The result can recommend stopping at the improved M package;
   it does not automatically allocate learning or a two-dimensional search.

The capture maximum is 8 + 16 + 158 + 60 + 14 = **256**. Counting each captured
execution as additional to its Summary scan gives at most **13,212 originals**;
all physical replays and independent reconstructions are extra counted native
work. Some roles may reuse a previously captured occurrence, reducing this
maximum without reducing declared coverage.
At the existing 4,800-tick per-episode hard limit, all Summary runs plus 256
captured originals and five extra native proof passes per capture require at
most 69,561,600 native steps. Pure reference/diagnostic scans have separate
query accounting and still consume the wall-time budget.

Proposed implementation belongs in separate analysis, timing-map runner and
report-adapter modules under `experiments/coast-ignite/`, with example build
bindings updated only as needed for source provenance. Reuse the maintained
common report renderer and native bridge. Keep `pd-control`, `pd-core`,
the generator, existing stopping rules, trained weights and old outputs intact.

## Budget, proof and report delivery

Proposed loop caps are two hours from its declared start, one native worker,
100 million total native steps, 14,000 original executions and 8 GiB of new
artifacts, with at least 16 GiB free before starting. Reserve all source epochs,
Summary records, up to 256 captures, their proof and one rich report copy.
The 4-arc pilot plus its 24 captured controls is capped at five minutes,
five million native steps including proof, and 1 GiB. These are proposed caps,
not an authorization to consume resources during this planning review.

Retain complete request/resource ledgers. Separate original, neutral replay,
controller reconstruction, scheduler/live-policy reconstruction and pure
reference-query counts. No optimizer steps run. Reserve the full phase before
starting it; software/resource stops retain partial evidence and do not become
physical misses. Any continuation requires explicit source/resource accounting
without changing cases, denominators or past outcomes. No automatic widening
of the timing domain, extra population or retry after an outcome is allowed.

The later HTML delivery should extend the established mission tree. Each arc
gets a small timing strip showing all 82 dispositions, fuel/time/contact-margin
plots and M/G markers, linked to rich details for captured points. Summary cells
must be visibly distinct from proof-backed captures. Keep main and diagnostic
populations separate; preserve full terrain, common navigation and existing
reports/selectors. The common tree expects dense, consistent approach slots:
do not put sparse captured arrays behind a global 82-approach selector. Use
uniform role/population projections and direct case/slot links for the maps;
never invent missing rich captures or mislabel their selected schedule.
Reuse a running report service only within a subsequently
authorized delivery; this plan authorizes no publication or server lifecycle.

Meaningful implementation checks cover integer timing endpoints, pre-ignition
contact censoring, never/NoPlan continuation, no state/fuel/clock reset, identical
coast prefixes, complete inventories, equality of Summary/capture/replay fuel,
best-point selection, disconnected windows, missing-neighbor labels and undefined
failure regret. Source validation uses the maintained developer gate plus
focused Python tests; browser acceptance checks the actual desktop/phone rich
report. Ordinary source checks must not require locally retained captures.

This planning pass needs only docs links, the relevant Node tests and diff
whitespace checks. Native campaigns, report generation and controller changes
are not part of documenting the proposal.

## Decisions supported by the result

| Measured landscape | Next direction to consider |
| --- | --- |
| Successful opportunities exist, but M/G miss narrow or disconnected sets | Improve the reliability of planner/controller handoff; review which feasibility information a planner should receive before allocating fuel learning |
| Broad successful interiors with little fuel variation | Retain M and prioritize reliability or broader mission capability; fixed-end fuel optimization has limited observed headroom |
| Material fuel headroom across broad successful interiors | Design a separate success-first fuel objective and frozen-policy comparison; new validation and untouched confirmation are required before a learning claim |
| Apparent savings concentrate at fragile early contacts | Resolve the intended timing/contact-margin contract prospectively, preserving every existing success and failure |
| Fixed-end maps leave the larger timing tradeoff unresolved | Propose a small separate ignition/reference-end grid before adding duration selection to RL |

The later two-dimensional probe would vary explicit reference-end instructions
on copies of the same physical inputs while retaining native integration and
immediate activation. Freeze its admissible end-time range, alignment, coverage,
deadline, counts and observation contract separately. A revised end time changes
both the generated reference and a network context feature; frozen-network
performance there is not qualified by the existing study. The first map is a
cross-section of that possible larger problem, not evidence of its optimum.

Further training, reward shaping, controller admission changes, general arrival
control, departure, terrain avoidance and default integration remain separate
decisions. The immediate deliverable is a trustworthy handoff/cost landscape
and a concrete reason to choose the next one of those tasks.
