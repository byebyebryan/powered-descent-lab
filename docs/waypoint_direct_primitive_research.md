# Direct-leg primitive research protocol

This document preserves the earlier analytical and controller-comparison
checkpoints. For the latest launch-aware known-flat nominal result, see
[Complete flat direct acceptance](waypoint_direct_complete_flat_acceptance_protocol.md).
That result uses new wrapper identities and does not revise the V2 certificates
or the source-contact findings recorded here.

This is an opt-in research pass for waypoint planning. Its first question is
whether the existing phase-structured V2 certificate can represent a direct
transfer over continuous flat, uphill, and downhill terrain with the same
vehicle and mission horizon as the five direct-route characterization cases.
`direct` means zero operational waypoint handoffs.

## Shared analytical setup

The five existing characterization inputs are the baseline: continuous flat,
uphill, downhill, bounded narrow ridge, and bounded broad mesa. The evaluator
copies only their scenario inputs. It does not use the ordinary planner's
decision or an observed controller trajectory to generate candidates.

Each case uses `evaluate_direct_bridge_case_v2`: a powered source bridge, an
exact discrete ballistic coast, and a powered terminal bridge. Its existing
terrain, thrust, throttle, slew, fuel, time, touchdown, and robustness screens
are retained. Candidate duration multipliers remain `[0.75, 1.0, 1.25, 1.5]`.
The vehicle, 120 Hz physics rate, and 9.81 m/s² gravity must exactly match the
characterization scenarios. The one shared effective V2 policy copies the
embedded V2 policy and sets `maximum_mission_time_s` to the characterization
missions' common 90 s horizon; its 10 s reserve and every other field stay
fixed. This policy is selected before inspecting candidate results.

A case has an analytical direct route if at least one candidate certifies.
The research choice among certified candidates is the shortest total time,
then candidate identity. The V2 evaluator's native selected candidate is
retained separately. Rejection means no candidate in this bounded family
certified; it does not establish physical impossibility.

The early gate requires a certified direct candidate in **each** continuous
flat, uphill, and downhill case. All five baseline rows are recorded. If the
gate fails, the terrain sweep stops and the failing candidate reasons become
the next research question.

Run the baseline with:

```text
cargo run -p pd-eval -- waypoint-direct-primitive-analytical \
  --output-dir /tmp/pd-waypoint-direct-primitive-analytical
```

The command writes `summary.json`, including the exact five probes, effective
policy, vehicle, all four candidate decisions per case, and semantic identity.
The initial 90 s run passed the early gate: flat certified 3 candidates,
uphill 2, and downhill 2. The narrow ridge and broad mesa controls each
certified 3 direct candidates. Shortest certified times were 31.0 s, 33.75 s,
35.5 s, 31.0 s, and 31.0 s in that order. The artifact identity was
`fnv1a64:d3fa6b24336f7c05` and its input identity was
`fnv1a64:ab2bc3937db28b43`.

## Conditional terrain sweep

After the early gate passes, generate 24 obstacles from the continuous flat
input with no change to pads, vehicle, initial state, policy, or cadence. The
obstacle center is at `0.35`, `0.50`, or `0.65` of the 800 m source-to-target
span. Its base width is `0.05` or `0.25` of that span, and its height is
`0.10`, `0.20`, `0.30`, or `0.40` of that span. Each obstacle is a trapezoid:
base edges at center ± half the base width; top edges at center ± one quarter
of the base width; top height equals the selected height above the flat
baseline. The source and target pad terrain points stay unchanged. Case order
is center, then width, then height in the order listed above.

Evaluate every cell's direct candidates first. Only a cell with no certified
direct candidate is eligible for a bounded one-waypoint composition. The
one-waypoint grid is fixed here before any sweep result is opened. Let `L` and
`R` be the obstacle's base edges and `T` its top height. Let `dx` and `dy` be
the horizontal and vertical extents of the shared vehicle's rotated hull
envelope with the policy clearance, each at least the policy minimum
clearance. Candidate waypoint positions are `(L−dx,T+dy)`,
`(L−2dx,T+2dy)`, `(R+dx,T+dy)`, and `(R+2dx,T+2dy)`, sorted by x then y.
Invalid or duplicate positions fail the setup instead of being silently
clipped. For each position, evaluate the complete four-by-four pair of the
shared duration multipliers, ordered by total ballistic leg steps and then
the two multipliers and step counts. This is a maximum of 64 compositions per
cell. The search stops at the first certified witness; exhaustion means the
bounded search is unknown. Each cell is classified `analytical_direct`,
`analytical_one_waypoint`, `unknown`, or `invalid_setup`; `unknown` is the
result when this finite search certifies neither topology.

Run the analytical sweep with:

```text
cargo run -p pd-eval -- waypoint-direct-topology-sweep \
  --output-dir /tmp/pd-waypoint-direct-topology-sweep
```

The first complete run produced `fnv1a64:1bcd5a3bd6c6da01`, bound to the
passing baseline identity `fnv1a64:d3fa6b24336f7c05`. All 24 cells were
`analytical_direct`; none entered the one-waypoint fallback. At every tested
center and width, the numbers of certified direct candidates by height
fraction `0.10`, `0.20`, `0.30`, and `0.40` were `3`, `3`, `2`, and `1`.
Thus this grid shows no direct-to-waypoint boundary or demonstrated waypoint
demand. It says nothing about obstacles outside the declared grid.

## Evidence order and decision

Seal analytical inputs, policy, candidates, and decisions before opening
controller outcomes. Then run the unchanged direct transfer controller on
every valid cell. Run the unchanged waypoint controller only for cells with a
certified one-waypoint route that its existing interface can represent. Keep
analytical certification, ordinary planner decisions, and controller outcomes
as distinct evidence.

The decision after the sweep is whether the shared bridge/coast primitive is
a credible direct-leg basis, whether a different tracker is needed, whether
the bounded composition demonstrates waypoint demand, or whether the result
remains unknown. This pass does not select a production planner or change any
default.

## Controller reveal and research decision

The sealed analytical sweep was compared with the unchanged `transfer_pdg`
controller through `waypoint-direct-controller-comparison`. Its source was the
exact `fnv1a64:1bcd5a3bd6c6da01` sweep artifact; each of the 24 scenarios
changed only the flat characterization mission's ID and terrain. The compact
comparison artifact identity was `fnv1a64:3569ec11f2645656`.

```text
cargo run -p pd-eval -- waypoint-direct-controller-comparison \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --output-dir /tmp/pd-waypoint-direct-controller-comparison
```

| Obstacle height / 800 m span | Certified analytical direct cells | Controller target landings | Controller crashes |
| --- | ---: | ---: | ---: |
| 0.10 (80 m) | 6 / 6 | 6 / 6 | 0 / 6 |
| 0.20 (160 m) | 6 / 6 | 6 / 6 | 0 / 6 |
| 0.30 (240 m) | 6 / 6 | 0 / 6 | 6 / 6 |
| 0.40 (320 m) | 6 / 6 | 0 / 6 | 6 / 6 |

All 12 crashes had negative sampled en-route hull clearance. The controller
observations are not executions of the analytical certificates, so their
failure does not invalidate those certificates. The finite analytical search
found no cell requiring a waypoint; no waypoint-controller run was eligible.
The bounded decision is to retain the shared bridge/coast direct primitive as
a research candidate and investigate how a controller could track a certified
direct profile on taller terrain. This grid does not justify adding an
operational waypoint or changing the default planner.

Two independent controller-comparison runs produced byte-identical summaries.
The legacy five-row characterization JSON, HTML, and SVG also remained
byte-identical after the research-only input and controller helper seams were
added. The integrated gate passed 616 workspace tests, strict Clippy,
formatting, and diff checks.

## Certificate-to-execution first gate (2026-09-23)

The next checkpoint froze the three continuous baseline inputs and three
centered, wide obstacle cells at heights 160, 240, and 320 m. It pinned the
baseline artifact `fnv1a64:d3fa6b24336f7c05`, the sweep artifact
`fnv1a64:1bcd5a3bd6c6da01`, and each shortest-certified research selection
before measuring commandability. The selected bridge samples were regenerated
from the stored probes; setup artifacts intentionally omit those runtime-only
samples. A focused regression test in `pd-eval` repeats this first gate.

At 120 physics ticks/s, the vehicle can rotate at most 0.013089969390 rad
from its upright starting attitude during tick zero. The selected profiles'
first powered source-bridge samples require:

| Frozen input | First thrust tilt (rad) | Tick-zero slew reachable? |
| --- | ---: | --- |
| Continuous flat | 0.011771414680 | yes |
| Continuous uphill | 0.006045287907 | yes |
| Continuous downhill | 0.030261510909 | **no** |
| Centered/wide obstacle, 160 m | 0.011771414680 | yes |
| Centered/wide obstacle, 240 m | 0.009446454319 | yes |
| Centered/wide obstacle, 320 m | 0.006477374512 | yes |

For downhill, the thrust magnitude maps to an applied throttle of
0.727785497777 after accounting for the simulator's fuel-before-acceleration
mass update; this is within the vehicle's 0.25–1.0 range. The failure is
attitude slew. Applying that first command to the authoritative plant on the
exact downhill characterization scenario yields 0.013089969390 rad of actual
attitude. Against the bridge's first post-step reference, position differs by
0.000015428794 m and velocity by 0.001851455299 m/s. This is an immediate,
measured mismatch, not a crash or landing outcome.

The frozen scenario uses a 60 Hz controller and holds its first command for
two 120 Hz physics ticks; that cadence cannot remove the tick-zero slew bound.

The V2 source attitude screen compares its first powered direction with the
0.15 rad safe-touchdown attitude allowance; its powered slew screen compares
powered directions *to one another*. Neither proves the transition from the
scenario's initial upright attitude to the first required thrust direction
within one physics tick. A second, slower certified downhill candidate
(`fnv1a64:8e9646e20efe007d`) requires only 0.009096490954 rad on its first
tick, but has not passed a full plant or controller replay. The frozen
shortest-time selection cannot be changed after observing this result.

The six-case execution gate therefore **stops at first-tick commandability**.
Full 120 Hz profile replay, 60 Hz held-command replay, feedback tracking,
held-out sweep reveal, and planner/default changes were not reached. This
result narrows the next work to an initial-attitude and cadence-aware
certificate/selection contract; it does not establish a need for a waypoint
or explain the tall-obstacle controller crashes.

## Nominal-to-plant follow-up (2026-09-23)

The subsequent [six-case nominal-to-plant pass](waypoint_direct_nominal_plant_protocol.md)
ran both frozen selection roles with one command rule at 120 Hz per-tick and
60 Hz two-tick-held cadences. Its identity-bound summaries repeated byte for
byte. All 18 simulator runs crashed on the first physics step while still in
source-pad contact; none reached an en-route or landing comparison. The
research-shortest downhill profile also showed the previously measured
first-tick attitude-slew mismatch. The immediate next research question is a
physically valid, explicit source-pad release transition, followed by a
separate later-profile representability check—not a waypoint/default change.
