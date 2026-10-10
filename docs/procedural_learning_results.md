# Procedural learning run record

[Documentation home](README.md) · [Execution contract](procedural_learning_plan.md) ·
[Experiment workflow](../experiments/rl/README.md) · [Ownership](architecture.md)

This October 10 run implements independent learning on complete native
source-pad-rest to target-pad landing missions. The existing controller and
planner are parked, unchanged comparators. The shared checkout is retained;
validated owned milestones are committed separately from the ten pre-existing
report/navigation/server paths.

Status: complete, October 10, 2026. Two terrain families, common campaigns/reports
and an independent PPO interface are implemented. All nine training allocations,
broad confirmations, frozen held-out comparisons and independent inventory replay
completed. Each selected v2 seed landed **0/2,048** held-out missions; the
reflected incumbent landed **1,973/2,048 (96.3%)** under explicit experimental
cap 24. The platform works; this learning setup has not established landing
competence. No current controller or default is replaced.

## Accepted platform and inputs

The neutral native bridge supplies physical observations, terrain queries and
ordinary `pd-run` episodes. The experiment owns features, action mapping, launch
primitive, rewards, samplers, optimizer and checkpoints. It imports no existing
controller or planner. The explicit incumbent application owns its concrete
solver dependency; common evaluation and reporting do not.

The old value-noise generator retains its pinned resolved-input and physical
identity. The second family composes layered regular/ridged Perlin, domain warp,
macro relief and sparse features. Its actual FastNoiseLite backend is named;
it is not represented as an exact Pylander noise implementation. Its computations
match an independently generated pre-port reference within `1e-10` at the tested
unquantized samples. Ordinary generator tests use the retained small fixture,
without a study binary or external game checkout.

Characterization scheduled 25,000 requests per family and repeated all requests
in reverse order. Both produced 25,000 unique physical inputs, with no exhausted
requests; four value-noise and six layered structural retries remain recorded.
These 100,000 generation executions establish determinism and input coverage,
not flight feasibility or controller success.

The declared collections contain 4,096 unique physical inputs with no cross-cohort
duplicates. Both families use the same physical/resource envelope and balanced
direction/relief strata. Distance, pad width, source position, terrain wavelength
and elevation vary without solver-outcome filtering.

| Role | Value-noise seed / count | Layered seed / count |
| --- | --- | --- |
| Training input reserve | 501101 / 256 | 501201 / 256 |
| Development | 501102 / 512 | 501202 / 512 |
| Validation | 501103 / 256 | 501203 / 256 |
| Held-out | 501104 / 1024 | 501204 / 1024 |

Training streams start at index 256 under the training seeds. Workers alternate
families and advance independent declared index schedules. Every consumed
proposal, structural rejection, physical identity and terminal training outcome
is retained. The nine completed allocations together consumed 41,000 unique
physical missions, rather than the sum of the overlapping per-run counts.
No consumed training identity overlaps development, validation or held-out.

## Learning evidence

Each design uses fresh seeds 801, 802 and 803, eight million requested transitions
per seed, and the same PPO architecture/optimizer. Each completed allocation
contains 8,003,584 transitions and 4,885 optimizer epochs. All source/native
identity checks and v3's additional installed-SDK checks passed. Their recurring
development panel contains 128 inputs
from both families; selection uses native summary success count, horizontal
target offset, then earlier checkpoint. These are summaries without saved-command
landing proof. The horizontal secondary metric omits vertical separation.

| Design | Actual transitions across three seeds | Selected panel successes | Median horizontal offset across 384 attempts |
| --- | --- | --- | --- |
| v1: distance-progress reward | 24,010,752 | 0 / 384 | 29.46 m |
| v2: added kinetic/contact potential | 24,010,752 | 0 / 384 | 23.28 m |
| v3: aligned discount and zero terminal potential | 24,010,752 | 0 / 384 | 206.08 m |

The full allocations total 72,032,256 transitions across nine independent runs.
The final reward revision corrects terminal accounting but did not improve the
declared development score. Its selected first-seed panel timed out on all 128
inputs; four supplementary native captures also timed out and replay exactly.

The declared design ordering selected v2, since all three designs had zero
successes and v2 had the lowest median horizontal offset. Selection was frozen
at **2026-10-10 21:35:40 UTC**, before the first held-out exposure. The freeze
retains checkpoint bytes, training receipts and matching original training source
and native-module bytes. All subsequent evaluation uses those checkpoint copies.

| Seed | Selected optimizer checkpoint | Frozen checkpoint SHA-256 |
| --- | --- | --- |
| 801 | 5,242,880 transitions | `f2b6a667b0df2f40694eec6e75140ec2065ca26e734fb8ef7521ff47eeca6e54` |
| 802 | 4,194,304 transitions | `a97e08b680ec84671d7f08140c503f6e83136ed64d6b2ca90697b0fd8cc0b73d` |
| 803 | 3,670,016 transitions | `c220ddcdfcea776bb3e72c94a38530efea24cc23b6e5f69a87ca82f3a1eb2a06` |

Broad confirmations evaluated each frozen seed on all 1,024 development and
512 validation inputs: zero native summary successes in each of the six panels,
with unchanged source/input/checkpoint bindings. These 4,608 repeated attempts
cover 1,536 unique inputs and carry no saved-command proof. They were not used
to re-rank designs or checkpoints.

The initial design mostly learned rapid transfer ending in crashes. Mean
completed distance shaping was about 63–79 reward units versus a crash penalty
of 20. Two captured development impacts had speeds around 80 m/s and replay
exactly. This exposed an objective problem rather than a reason to introduce
mission-specific control rules.

The braking revision learned predominantly horizon stops. Its selected panels
for seeds 801 and 802 contain 117 and 112 timeouts, respectively, out of 128.
Eight additional captured developmental flights ended with slow endpoint motion
at the 90-second horizon, without touchdown; all replay exactly. Horizontal
progress or reduced impact speed cannot substitute for native landing success.

The final revision uses the same state potential and physical/action/observation
controls. It aligns shaping to PPO's 0.999 decision discount and zeros terminal
potential for every intrinsic stop. The discounted shaping sum is constant for
all finite trajectories from the same initial state. Tests verify that identity
and unchanged native commands, clocks and physical results under identical
actions. An archived-source check also verifies exact original reward behavior
after extracting the shared reward hook. A real 65,536-transition pilot passed
source/native and installed-SDK integrity checks before the full allocation.

## Evaluation and preservation

All six frozen-policy held-out campaigns completed. Each seed received the same
2,048 physical inputs, 1,024 per family. The 6,144 attempts are three repeated
policy evaluations of that common population, not 6,144 unique missions.

| Seed | Family | Verified target landings | Native timeouts | Native crashes |
| --- | --- | --- | --- | --- |
| 801 | Value noise | 0 / 1024 | 982 | 42 |
| 801 | Layered | 0 / 1024 | 921 | 103 |
| 802 | Value noise | 0 / 1024 | 950 | 74 |
| 802 | Layered | 0 / 1024 | 884 | 140 |
| 803 | Value noise | 0 / 1024 | 963 | 61 |
| 803 | Layered | 0 / 1024 | 902 | 122 |

Every policy attempt has a native capture and rich report. A fresh process,
importing no learning library and loading no checkpoint, independently verified
all 9,216 executed captures across the ten policy/incumbent campaigns. Integrity,
physical-prefix replay, complete bounded-episode replay, original input identity,
descriptor and ledger/report projection checks passed. The 1,024 unsupported
raw-incumbent attempts remain visible; all 10,240 scheduled attempts are
accounted for, with no preparation/execution/capture failures.

| Approach | Verified landings / scheduled | Unsupported | Independently replayed / executed |
| --- | --- | --- | --- |
| PPO v2, seed 801 | 0 / 2048 | 0 | 2048 / 2048 |
| PPO v2, seed 802 | 0 / 2048 | 0 | 2048 / 2048 |
| PPO v2, seed 803 | 0 / 2048 | 0 | 2048 / 2048 |
| Incumbent raw | 984 / 2048 | 1024 | 1024 / 1024 |
| Incumbent reflection | 1973 / 2048 | 0 | 2048 / 2048 |

Raw support is the rightward slice: 490/512 value-noise and 494/512 layered,
or 984/1,024 (96.1%) on that explicitly labeled subset. Reflection lands 989/1,024
value-noise and 984/1,024 layered. All 1,024 rightward missions preserve raw
projected native outcomes, stop reason, execution validity, flight time and fuel
exactly under the reflection adapter. Its 75 non-landings are finite solver
stops while still flying, with valid bounded native replay; they are not landing
proof or native horizon completions. Neither incumbent campaign has a crash.

Independent replay also measures the policy's actual timeout endpoints:

| Seed / family | Timeout endpoints | Median horizontal offset | Median height above upright target touchdown | Median speed |
| --- | --- | --- | --- | --- |
| 801 / value noise | 982 | 30.45 m | 63.87 m | 0.51 m/s |
| 801 / layered | 921 | 32.58 m | 63.05 m | 0.47 m/s |
| 802 / value noise | 950 | 22.89 m | 56.09 m | 0.77 m/s |
| 802 / layered | 884 | 24.55 m | 55.56 m | 0.76 m/s |
| 803 / value noise | 963 | 19.57 m | 60.02 m | 1.74 m/s |
| 803 / layered | 902 | 21.64 m | 59.99 m | 1.67 m/s |

These are medians within each campaign's native timed-out subset, not averages
over crashes or a new checkpoint-selection metric. Height is final reference
position minus pad surface minus the vehicle's upright touchdown-base offset;
it is not an attitude-dependent hull-clearance measurement. In total, 5,602
policy flights timed out and 542 crashed. The slow, elevated endpoints explain
why horizontal selection progress did not imply touchdown readiness.

Common campaigns bind immutable collection inputs once across a run and bind
each selected input per episode. Append-only exposure receipts are recorded
without treating another campaign's later receipt as physical input mutation.
There is no repeated whole-manifest scan or whole-collection hashing per episode.
SDK code/binaries are bound across final policy evaluation, including loading
the checkpoint. Earlier training retains its original source/native checks and
dependency lock; the final revision additionally binds installed SDK bytes.

Every scheduled attempt remains visible, including unsupported, invalid-input,
initialization, execution and capture failures. Actual fault probes retained one
changed input alongside its valid neighbor, and recorded two initialization
failures for a bad checkpoint. Neither silently shrank its denominator.

The raw incumbent explicitly rejects leftward inputs. Its separately named
global reflection adapter plans a reflected physical input and executes the
transformed commands on the original mission. Both use explicit combined-safety
configuration and experimental cap 24; existing defaults remain unchanged.
Early finite solver stops preserve their original executed prefix and budget.

Common individual reports retain the rich native template. Typed campaign and
paired-comparison views use the common batch shell and evidence navigation.
The comparison rejects different physical inventories or repeat counts, includes
unsupported inputs and gates combined landing counts on campaign integrity.
Browser layout review passed on development reports and the final five-approach
comparison. All 9,267 local aggregate/report links resolve and all 9,216 executed
attempts have rich detail reports. Final inventory replay is independent of the
policy, rather than only the earlier sixteen-capture pilot. This is native/static
report evidence; no game-host deployment or human flight acceptance is claimed.

The maintained 18-check development gate and thirteen Python contract tests pass
on the final reward source milestone. The final preservation receipt checks all
120 starting source files across core, runtime, controller, planning math, solver
and the two existing applications, all 84 starting fixture files and the ten
other-writer paths. Their bytes remain unchanged. No historical evidence is
rewritten, and no existing report refresh/publication, server lifecycle change
or default promotion occurs.

## Retained evidence

The create-only run root is
`outputs/procedural-learning/run-20261010T190900Z/`. Important records include
`baseline.json`, `cohorts-plan.json`, `cohort-validation.json`,
`native-build-receipt.json`, retained native-build sources and installed extension,
all training directories/checkpoints/worker logs, `final-audit.json`,
`reward-extraction-equivalence.json`, `independent-pilot-verification.json` and
the maintained gate logs. The retained launcher scripts record actual commands
and exit/timing results. The experiment workflow documents reproduction from
source and new inputs; ordinary checks never require this output archive.

- [Final paired report](../outputs/procedural-learning/run-20261010T190900Z/held-out-evaluation/report.html): all five approaches and family/direction/relief coverage, with campaign and flight navigation.
- [Independent verification](../outputs/procedural-learning/run-20261010T190900Z/held-out-evaluation/independent-verification.json): replayed inventory and native endpoint diagnostics; no checkpoint loaded.
- [Final audit and preservation receipt](../outputs/procedural-learning/run-20261010T190900Z/final-preservation-and-comparison.json): matched physical inventories, attempt accounting and original source/fixture/other-writer hashes.

`report-link-inventory.json`, `raw-reflection-rightward-preservation.json` and
`final-comparison-preview.png` retain the link, adapter-preservation and browser
checks. `held-out-evaluation/commands.json` and `execution.json` record all ten
actual commands, successful exits and measured wall times.

## Interpretation and next experiment

This loop establishes independent strategy ownership and a working procedural
learning/evidence pipeline. It does not establish a competent learned lander.
The nine bounded allocations and two general reward revisions are complete;
there is no further reward search or controller repair inside this contract.
The positive incumbent result shows that many inputs in this declared population
are solvable. It does not establish another generator's coverage, arbitrary
terrain reliability, or a directly comparable rate to the old 1k campaigns.

The [next experimental recommendation](procedural_learning_next.md) proposes a
physical distribution audit, a controlled curriculum over complete procedural
missions and a separately bounded control-interface investigation. Keep corrected
v3 objective accounting as the next baseline and vary one design factor at a
time. The observed held-out population can become a later regression panel,
retaining its original role and exposure history; new generalization claims
require fresh held-out seeds. This recommendation is not a launched allocation.
