# Procedural learning run record

[Documentation home](README.md) · [Execution contract](procedural_learning_plan.md) ·
[Experiment workflow](../experiments/rl/README.md) · [Ownership](architecture.md)

This October 10 run implements independent learning on complete native
source-pad-rest to target-pad landing missions. The existing controller and
planner are parked, unchanged comparators. The shared checkout is retained;
validated owned milestones are committed separately from the ten pre-existing
report/navigation/server paths.

Current status: the two terrain families, common campaigns/reports and independent
PPO interface are implemented. Initial and braking-reward allocations are
complete. The final declared reward revision is running. Checkpoint freezing,
broad confirmation and the final native held-out comparison remain pending.
This record does not yet establish goal completion or a learned landing policy.

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
is retained. The first six completed allocations together consumed 41,000 unique
physical missions, rather than the sum of the overlapping per-run counts.
No consumed training identity overlaps development, validation or held-out.

## Learning evidence so far

Each design uses fresh seeds 801, 802 and 803, eight million requested transitions
per seed, and the same PPO architecture/optimizer. Each completed allocation
contains 8,003,584 transitions and 4,885 optimizer epochs. The first six source
identity checks passed. Their recurring development panel contains 128 inputs
from both families; selection uses native summary success count, horizontal
target offset, then earlier checkpoint. These are summaries without saved-command
landing proof. The horizontal secondary metric omits vertical separation.

| Design | Actual transitions across three seeds | Selected panel successes | Median horizontal offset across 384 attempts |
| --- | --- | --- | --- |
| v1: distance-progress reward | 24,010,752 | 0 / 384 | 29.46 m |
| v2: added kinetic/contact potential | 24,010,752 | 0 / 384 | 23.28 m |
| v3: aligned discount and zero terminal potential | Running | Pending | Pending |

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
Browser layout review passed on saved local development reports. An independent
process also replayed sixteen pilot captures without importing a learning library
or loading a checkpoint. This is native/static report evidence; no game-host
deployment or human flight acceptance is claimed.

The maintained 18-check development gate and thirteen Python contract tests pass
on the final reward source milestone. All starting core, runtime, controller,
planning-math and solver source bytes, all 84 starting fixture files and the ten
other-writer paths remain unchanged. No historical evidence is rewritten, and no
report refresh/publication, server lifecycle change or default promotion occurs.

## Retained evidence

The create-only run root is
`outputs/procedural-learning/run-20261010T190900Z/`. Important records include
`baseline.json`, `cohorts-plan.json`, `cohort-validation.json`,
`native-build-receipt.json`, retained native-build sources and installed extension,
all training directories/checkpoints/worker logs, `audit-through-v2.json`,
`reward-extraction-equivalence.json`, `independent-pilot-verification.json` and
the maintained gate logs. The retained launcher scripts record actual commands
and exit/timing results. The experiment workflow documents reproduction from
source and new inputs; ordinary checks never require this output archive.

Final evaluation will use the winning design's three frozen checkpoints on the
same 2,048 held-out inputs, alongside raw and reflected incumbent campaigns.
Broad development/validation are confirmations after freezing, never another
selection search. Further reward revisions are outside this run's declared
allocation. Final results and the next experimental recommendation will replace
the pending status only after those campaigns and independent replay finish.
