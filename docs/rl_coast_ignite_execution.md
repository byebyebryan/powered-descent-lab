# Coast and ignition execution

[Plan](rl_coast_ignite_plan.md) · [Documentation home](README.md)

October 22, 2026. The owner requested a goal loop after reviewing the researched
plan. This allocation implements and conditionally runs that bounded experiment.
It does not reopen direct-control RL or generalize the landing controller.

The create-only run root is
`outputs/coast-ignite/loop-20261022T173645Z`. Its workspace baseline records all
pre-existing tracked/untracked bytes, HEAD and dirty status. Preserve them.
Review and commit owned implementation slices locally; no push or server changes.

## Limits and release rules

One native worker, sequential environments, four hours from goal creation;
32 GiB new artifacts and at least 50 GiB free. A phase's limit is subordinate
to the remaining outer allocation. Software work has a 60-minute target.

| Phase | Initial ceiling | Release |
| --- | --- | --- |
| Software | Native unit/parity/proof and Python update tests | Contracts and maintained gate pass |
| Cost pilot | 8 development arcs × 8 schedules; 5 min, 1.5 million native steps, 1 GiB | Measure original/proof/storage cost before later phase limits are frozen |
| Timing map | 12 arcs × 82 schedules; proposed 10 min, 5 million steps, 256 MiB | Native witness results, decision resolution and useful timing windows reviewed |
| Readiness | 256 sampled episodes, then one PPO update; proposed 10 min, 2 million steps, 256 MiB | Positive outcomes on at least two arcs, intact exploration and finite noncollapsed update |
| Learning | 3 seeds × 131,072 decisions; 30 min and 50 million native steps per seed | Fixed validation/checkpoint schedule; source and task freeze |
| Confirmation/report | Proposed 50 min, 60 million steps, 24 GiB captures + 4 GiB presentation | Selected validation packages verified before releasing 600 fresh arcs once |

Freeze exact later phase limits in create-only JSON receipts after the cost
pilot, before execution. Count original physics, admission work, neutral replay,
controller/model reconstruction, optimizer updates and all repeated evaluations.
Faults stop execution; ordinary native misses remain valid measured outcomes.
Failed readiness closes this allocation without training or fresh-data release.

## Progress

- Goal opened at main `b0bfb65`; no source or native campaign changed beforehand.
- Implementing controller-owned coast, same-tick ignition and a separate approach
  binding. Accepted reference/actuation arithmetic and neutral crates stay intact.

Before the first flight, the cost-pilot ceiling was set from the worst-case
64 original episodes plus three neutral replays and two reconstructions each:
1.5 million native steps and 1 GiB, within the unchanged outer limits.

## Software acceptance

The maintained gate passed all 19 checks with `RUST_TEST_THREADS=1`. An initial
parallel run hit the existing report-navigation missing-pinned assertion; that
test passed independently and in the complete serial run. No report-navigation
source was changed. Native coast integration has three geometry/continuity/
rejection tests, and the locked Python approach has four hazard/returns/model/
proof tests. Epoch B also passed all four Python tests against its own bridge.

The accepted reference, paired actuation, checked landing controller and neutral
Python bridge remain byte-identical to the goal baseline. Epoch A was a software
fixture snapshot only. The measured release uses `source-epoch-b/` (329 copied
source/fixture files) and `execution-allocation-v2.json`. The final preflight
changes added compact bound report evidence and byte accounting over the whole
run root; full rich flight views and access to the sealed captures remain.

## Measured preflight and retained interruption

Epoch B completed the 64-slot cost pilot in 4.96 s, retaining 187,909,008 bytes.
Every slot passed neutral physical replay, controller reconstruction and policy
reconstruction. Its 109,750 original native steps plus three neutral passes and
two reconstructions total 658,500 known native steps, within the frozen pilot cap.

The timing map completed all 984 matched schedules plus 12 sampled-prior episodes.
All 12 nominal witness timings had native success. Successful windows contain
21–61 policy slots; immediate ignition succeeds on 6/12 arcs and the best common
fixed grid time on 10/12. Those are exposed native development outcomes, not a
fresh reliability claim. The 256 sampled readiness episodes have 107 distinct
positive native outcomes. One complete PPO smoke update takes 2,048 decisions
and 64 optimizer steps, remains finite and noncollapsed, and changes the weights.

The first three-seed run stopped at checkpoint zero because metrics accessed
SB3's logger before `learn()` initializes it. No training-seed optimizer update
or training episode occurred; 64 checkpoint-zero validation episodes did run.
The complete fault, source epoch, checkpoint and cost counters remain under
`campaign-b/`. This is an implementation fault, not a failed landing or learner
result. Fresh inputs were not generated.

The reviewed repair returns empty metrics before learning and admits an explicit
continuation only for this closed zero-update fault. It verifies unchanged
controller/generator/features/hazard sources, the original policy-weight digest,
all retained source bytes and all 64 validation inputs. Epoch C changes only the
runner among participating runtime sources. Prior validation and seed costs are
subtracted from the original phase limits; the four-hour and byte caps remain.

## Frozen learning and fresh result

All three declared seeds completed 131,072 policy decisions. Their selected
validation checkpoints are seed 281101 at 65,536 decisions (61/64), 281102 at
131,072 (64/64), and 281103 at 98,304 (64/64). The predeclared tie rule selects
281103 at 98,304 for fresh evaluation. Actual training optimizer steps are
4,070, 4,096 and 4,014; the readiness update adds 64 separately. All 256 selected
validation policy/heuristic captures pass complete proof and agree with selection.

The selected non-oracle heuristic is a fixed three-second delay: 53/64 exposed
validation successes, tied with the four-second time-to-contact heuristic and
selected by the prospective rule. No fresh outcomes reselect either package.

| Frozen package | Verified fresh target landings | Exact two-sided 95% interval |
| --- | --- | --- |
| Selected RL | 589/600 (98.17%) | 96.74–99.08% |
| Selected fixed-delay heuristic | 446/600 (74.33%) | 70.64–77.78% |
| Immediate ignition | 255/600 (42.50%) | 38.51–46.57% |
| Never ignite | 0/600 | 0–0.61% |

All 2,400 scheduled fresh episodes completed with neutral physical replay,
controller reconstruction and frozen-model/rule reconstruction. The primary
paired comparison has 146 RL-only wins and three heuristic-only wins: 23.83
percentage points lift, one-sided exact p `7.727460381519204e-40`. The declared
advantage criterion passes. This is a result for one validation-selected package
on the supplied-arc task, not seed-wide fresh qualification or generalized flight.
The previous controller-only 99% criterion does not apply to this different task.

Ignition varies over 71 slots from 0 to 7.1 s, median 3.6 s. The 11 RL misses are
seven rejected requests followed by passive-coast crashes and four crashes after
an accepted burn. Native contact remains authoritative; no rejected request was
retried. Initial-phase breakouts are descriptive subsets of the same fresh set:
ascending 186/193, near-apex 12/12, descending 391/395 RL successes.

The original fresh result uses `counts` for package successes, which shadows its
resource-counter field. The immutable result remains intact. A separately saved
`confirmation/resource-accounting-supplement.json` reconstructs work from all
2,400 hash-checked sealed captures and decision records, the frozen proof
implementation and all retained phase receipts. Fresh evaluation used 3,845,473
original native steps and 23,072,838 steps including three neutral replays and
two reconstructions. The whole measured allocation has 61,439,988 known native
steps. A regression fix preserves `resource_counts` independently in future
receipts; it changes neither the frozen epoch nor its outcomes.

## HTML delivery and next boundary

The [complete HTML report](http://starship:8000/coast-ignite/loop-20261022T173645Z/campaign-c/learning/report-final-v2/report.html)
groups all four policy attempts under each of 600 physical arcs, using the
established mission tree, pagination, filters, comparison previews and rich
detail template. The original full tree and validation flights remain linked.
The display-only final edition adds verified outcome tables, all-seed learning
curves, miss inspection and a default camera fitted to the saved flight, pad and
reference. Full terrain, commands and sample payloads remain unchanged.

The browser plugin had no connected browser. A separately owned disposable
Chrome checks desktop/phone layouts, all 600 unique paginated arcs, four attempts
per arc, rich plots and matched back/next navigation. The existing report server
is reused without lifecycle changes. Final screenshot review and delivery/hash
checks are recorded below before closure.

Review the 11 retained misses before changing the task or controller. Further
learning, duration selection, attitude preparation, generalized arrival or
terrain work needs a separate bounded proposal and new fresh inputs. No default
promotion or push is part of this loop.

Final acceptance: all 2,400 capture/decision hashes and complete proof receipts
were independently checked, and all 600 physical identities retain four policy
attempts. Desktop/phone browser acceptance and screenshot inspection pass for
`report-final-v2`: 12 arcs per page, all 600 reachable exactly once, rich plots,
matched back/next navigation, focused flight camera, intact full-terrain data,
and zero browser errors. The final presentation is 3,237,953,998 bytes and its
source copy is retained under `presentation-source/`. The report's served bytes
and final workspace preservation checks are recorded in the closure receipt.
