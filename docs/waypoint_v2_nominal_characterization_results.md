# Unified nominal characterization results

## Verdict

The 2026-10-01 research checkpoint is complete, but the studied family is **not
ready for runtime replacement**. It produces independently replayed free-space
target-plane landing witnesses for all 39 retained airborne states, including
the four previous policy-2 NoNominal handoffs. It succeeds on only two of eight
clear ground starts. The existing source-rest planner's retained 8/8 uncut
results must not be replaced by this 2/8 experiment.

This rejects the particular ground preparation/acquisition/entry construction,
not the direct-first architecture, the vehicle's ability to make these flights,
or the idea of a shared state-aware constructor. The six ground failures occur
in analytical screens after successful liftoff, not in a first-step crash.

The [protocol](waypoint_v2_nominal_characterization_protocol.md) defines the
family; the [separate primary review](waypoint_v2_nominal_characterization_review.md)
records the integration no-go and the next bounded diagnostic. No new runtime
policy, full waypoint matrix, commit, push, deployment or default promotion is
part of this checkpoint.

## Inputs and measurement

The corpus binds eight clear starts, all 27 actual local handoffs from the final
policy-2 matrix, and twelve historical airborne captures. The 27 handoffs are
25 ordinary and two diagnostic boundaries; repeated handoffs from one mission
are not independent mission trials. Eight fresh synthetic conditions are
reported separately.

All 39 airborne starts are reproduced from fresh simulations through their
complete original commands. All compared snapshots match exactly, including
clock, held command, fuel, attitude/rate, outcomes and accumulated metrics.
Expected snapshots are comparison values, never execution seeds. Each ground
preparation program is also independently replayed from fresh H0.

The sealed corpus SHA-256 is
`c570da97f46cdaa5daf70238964d52afaad663107f42e5ef3205a7d186a7c128`.
The evaluator requires these exact bytes and verifies all 107 source bindings
before and after running. The builder includes 46,238 original held-pair prefix
updates. Every original deadline remains tick 9600, with 120 Hz physics and
60 Hz control. No retained terrain, vehicle, prefix or old evidence is edited.

## Coverage without mixing evidence levels

The first witness below is the first terrain-blind ranked candidate. Other
shortlisted witnesses are research observations, not terrain-aware alternatives
that the planner is allowed to select.

| Retained population | Inputs | Free-space witness rows | First-ranked raw terrain landings | First-ranked landings with no noncontact reserve violation |
| --- | ---: | ---: | ---: | ---: |
| Clear ground starts | 8 | 2 | 2 | 2 |
| Actual local handoffs | 27 | 27 | 20 | 18 |
| Historical captures | 12 | 12 | 12 | 12 |

Considering any of the three shortlisted witnesses raises the local raw-landing
count to 22 and the local landing-plus-reserve count to 21. Neither is planner
coverage: selecting another trajectory because of terrain would violate this
study's contract. Seven first-ranked local proposals contact obstructing terrain;
two more land but violate the noncontact body reserve. These need terrain audit
and potentially another clearing piece, not a terrain-dependent nominal rank.

There are 118 retained supplied-command witnesses, all with safe first
target-plane contact at the claimed proposal endpoint and independent replay
agreement. One hundred also land on their real terrain. The synthetics add
15 witnesses, twelve real-terrain landings, seven successful rows and one finite
low-fuel miss. Thus the CLI totals of 133 target-plane witnesses and 112 raw
terrain landings count witnesses, not successful missions or 55 independent
planner trials.

## The four former NoNominal states

The first terrain-blind ranked witness in each case lands on the unchanged real
terrain, with no measured noncontact body-reserve violation:

| Actual handoff | Selected acquisition | Entry angle | Finish tick |
| --- | --- | ---: | ---: |
| Late ridge H2426 | Natural vertical profile; lateral acquisition | 80.04 degrees | 4080 |
| Late plateau H2446 | Natural vertical profile; lateral acquisition | 81.75 degrees | 4062 |
| Successive rising H2804 | Natural vertical profile; lateral acquisition | 78.53 degrees | 4656 |
| Successive plateaus H3702 | Upward shaping; 26.75 m/s upward impulse | 78.36 degrees | 5476 |

The first three need no acquisition lift: lateral timing/braking plus terminal
capture is sufficient in this family. This is executable evidence against
interpreting the old 56-combination exhaustion as physical impossibility. The
fourth is recoverable with vertical shaping. Terrain is not an acquisition or
rank input in either case.

These are new suffixes from legitimately reproduced old handoffs. A future
runtime constructor will generate different initial flights and handoffs; this
does not prove that its complete waypoint loop lands 16/16, or even 13/16.
The unchanged implementation's last full matrix remains 8/8 clear and 12/16
ordinary, below its declared usability floor.

## Why the ground rows miss

All eight ground adapters finish at H204, or 1.7 s, without a source contact.
They reach 16.624 m COM height above the source plane and 13.640 m/s upward
speed, spending about 84.15 kg fuel. Fresh replay agrees. The four flat and two
uphill starts then produce no admissible estimate and no physical witness
attempt; only the two downhill starts pass.

Seven of thirteen seeds in each flat row reject excessive acquisition demand.
Other entry screens reject non-descending geometry, lateral reversal, or the
coupled terminal thrust bound. Uphill rows have ten seeds because there is no
natural unpowered crossing; they still examine upward-shaping acquisitions.
The closest non-reversing terminal bounds for the six misses are approximately
18.288–18.320 m/s² against their declared 16.981–17.075 m/s² limits.

This does not establish a physical thrust deficit. The bound is conservative,
and the study restricts upright prep, acquisition shape, virtual times and entry
sampling. It has not isolated which restriction is decisive. Loosening the
0.925 factor after seeing these results would be outcome tuning, not validation.
There is already contrary evidence of successful uncut ground flights from the
existing constructor. Use those as controls for the next diagnostic.

## Numerical and one-sided findings

Across the 133 materialized witnesses, maximum acquisition position error is
5.94 micrometres, velocity error is 1.48e-12 m/s, and absolute fuel-estimate error
is 3.01e-11 kg. Maximum entry position/velocity errors are comparable. These
measure agreement for this discrete constant-acceleration realization and its
paired-throttle inverse, not a globally calibrated estimator margin.

All witness finish ticks are between 1754 and 6876. Actual entry angles range
from 17.66 to 87.16 degrees. This distribution is not a newly accepted minimum
angle: the screen tests descending geometry, coupled capture demand, alignment,
fuel, deadline and lateral shape, and execution tests the existing landing rule.
Powered braking can extend beyond the virtual ballistic intercept; the actual
finish must still meet the original deadline.

Among first-ranked retained airborne witnesses, 25 use zero acquisition, three
preserve the natural vertical profile while correcting lateral motion, and eleven
add upward shaping. Safe-descent and lateral-miss synthetics retain zero
acquisition. The steep synthetic needs shaping under this finite coupled screen;
its angle alone did not establish safe target capture. The low-fuel condition
rejects honestly. A separate steep/high-energy unit condition also rejects,
showing why angle alone cannot certify landing.

The overshoot synthetic successfully brakes and reverses during acquisition,
then moves toward its target and lands without terminal overshoot/return. Forward
is defined from the current position to the target, not an archived source frame.
No post-result rule was added to force this synthetic to fail. This does not
extend the unchanged local-clearing loop's backward-flight support.

The maximum observed seed count is thirteen, turn-consistency updates two,
entry screens four per seed, and witnesses three per row. These are finite
construction budgets, not a measured runtime latency guarantee. No complete
V2 planning-time benchmark or new interior-terrain twin gate is claimed.

## Review corrections and repeatability

Before accepted final runs, review corrected thrust-versus-acceleration units,
optional uphill natural crossings, natural-time zero acquisition, actual powered
arrival timing, interior lateral-reversal extrema, odd endpoint replay, and
integrity classification. The real-terrain audit now compares ordinary execution
against a neutral copy through the consumed prefix and uses raw incoming contact,
before ordinary landing handling snaps position/velocity. It does not miscount
that adapted landed state as an airborne 5 m reserve violation. Core contact and
landing predicates are unchanged. Synthetic row integrity failures contribute
to the artifact's global failure status.

Accepted roots are
[final_verified_a](../outputs/research/waypoint_v2_nominal_characterization_20261001/final_verified_a/summary.json)
and
[final_verified_b](../outputs/research/waypoint_v2_nominal_characterization_20261001/final_verified_b/summary.json).
Their complete summary bytes match without timing/path exclusions. Artifact
identity is `fnv1a64:44ab26564225262d`, repeat payload identity
`fnv1a64:b615492a9b56de96`, and summary SHA-256
`0238b3632a71c8ae5f599fabfca0a7ca303584669b91b9c40f99b3f7fa397d0b`.
The earlier `smoke_a` is preliminary, not the accepted source or audit.

Final evaluator-module SHA-256 is
`362706bdef0f9590ad1973b7c899e9ac6cb2a2a7aa48e45d303f46559a5bb89d`;
corpus builder SHA-256 is
`6880eaf1f93b57ab5fd3e710756c64025f0d3b3f3e73d90ba0796888c30bb0ad`;
protocol SHA-256 is
`4d086113e82ddb12f771139402621e014ed21a090341960785d8423eb4f01204`.

Reproduce with the retained local archives available; generated evidence is
ignored by Git. A fresh checkout alone does not contain those archives. Choose
new output paths because the builder and evaluator refuse overwrite:

```sh
rtk node scripts/waypoint_v2_nominal_characterization_corpus.mjs --output NEW_CORPUS_JSON
rtk cargo build -p pd-eval
rtk proxy target/debug/pd-eval waypoint-v2-nominal-characterization \
  --corpus NEW_CORPUS_JSON --output-dir NEW_OUTPUT_ROOT
```

The integrated workspace passes 885 tests with one existing ignore; strict
all-target Clippy, formatting and whitespace checks pass. The twelve focused
characterization tests, required CLI-argument test, corpus validation/tamper
checks and old V2 fake-CLI harness pass. Frozen-suite input-only preflight still
accepts thirty inputs and correctly rejects two diagnostics; it creates no
simulation and evaluates no flight acceptance.
All eighty relative links in the seven affected status/design/research documents
resolve. A separate artifact check verifies all 133 command schedules, actual
incoming-state bindings, finite budgets, deadline/contact/replay flags and the
complete final-payload byte comparison.

All 71 fixture files and 784 files in the monitored retained policy-2/historical
evidence roots match their pre-pass byte hashes. That is byte preservation, not
a fresh rerun of every old physical matrix. The fresh 39-prefix reproduction is
the current consumed-prefix physical check. Monitored roots are the practical
suite's `final_policy_2_a`, `final_policy_2_b` and `preservation_final`, and the
airborne canary's `final_contact_v1_b`. Luna built the bounded corpus and
evaluator; the primary owned the research family, review corrections, integrated
validation and verdict. The worker-loop and documentation workflows kept the
checkpoint separate from runtime and mission acceptance.
