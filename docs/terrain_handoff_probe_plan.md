# Bounded handoff timing/selection diagnostic

Date: 2026-10-07. Approved diagnostic scope, not a production planner fix.

## Question and evidence

The [paired relaxed-cap capture](terrain_cap_sweep_results.md) contains 98
`NoNominal` stops. All fail analytical seed/entry screens before any physical
nominal witness; this does not prove that terrain or physics makes them impossible.
In `983/967/955`, substantial locally safe terrain clearance is already achieved
before the planner hands control back. The current progress gate requires
passing the original predicted conflict's x coordinate plus a body diameter;
it can therefore postpone replanning while the craft coasts toward the target.

Among the 98 stops, 59 selected handoffs have nonnegative estimated horizontal
braking room and 39 have negative room. Forty-four have an already accepted
alternative with lower forward speed, lower downward speed and no less altitude;
48 have a higher positive-room alternative. These are overlapping descriptive
groups, not proofs of landing feasibility or expected recoveries.

Separate three explanations before making another heuristic: late replanning,
clearing-row choice, or an acquisition capability missing even from better states.
Keep the gamified, piecewise contract: terrain-blind nominal construction, terrain
audit second, one obstruction-clearing maneuver, then actual-state replanning.
Do not require every waypoint to prove a direct landing suffix.

## Frozen experiment

The [machine contract](../studies/terrain_profiles/handoff_probe_plan.json) pins
the baseline manifest/receipt, seven exact scenario/flight byte hashes, cycle
indices, row identities and clocks. SHA-256:
`fef2c150ac15fd1d35d1795b9a17ced02afac4c17873226d48fd517f5bc8ad0e`.

| Subject | Original H step | Earliest settled coast step | Max-room accepted row H step |
| --- | ---: | ---: | ---: |
| random-983 | 3112 | 2428 | 3124 |
| random-967 | 3218 | 2882 | 3550 |
| random-955 | 3150 | 2964 | 3160 |
| random-024 | 3062 | 2828 | 3118 |
| random-516 | 2756 | 2274 | 2756 |
| random-271 airborne control, cycle 3 | 4226 | not probed | not probed |
| random-000 original-rest control | 0 | not probed | not probed |

Run original states first (seven), then five earlier states on the **same selected
maneuver**, then five preselected alternative rows. The alternative is maximum
finite remaining braking room among already accepted rows, ties by row identity;
no nominal result influences this choice. Finally repeat `967-early` and
`983-early`: **17 primary queries + 2 fixed repeats = at most 19 probes**.
One worker, 120 seconds per probe and 900 seconds for measured collection.
No retries, replacement cases, parameter search or additional scenarios.

## Native reconstruction and proof

- Authenticate the complete retained cap sweep before collection. Use exact
  original scenario bytes; do not synthesize terrain, move pads or edit vehicle,
  fuel, clocks, controllers, templates, ranking or planner defaults.
- Start the actual ordinary plant at its original source. Reexecute completed
  segments and the relevant fixed nominal prefix to the original row's E.
  Recompute that one native clearing row under its original goal. For original
  and early arms, require the complete proposal to equal the saved selected row;
  for alternatives, require its accepted full H state and fuel burn to match.
- Advance that actual plant to the frozen query clock. Saved snapshots are
  comparison evidence, **never restart inputs**. Reprove the entire consumed
  command/action prefix from the source with unchanged segment guards and replay.
- Earlier queries require upright, settled, zero-throttle coasting, exact forward
  trace agreement, the unchanged 5 m reserve at every physical step over the
  next two seconds, and supported-state checks at every command boundary.
  The original maneuver's later accepted H remains its clearing witness. An
  earlier query is **not** a production-admitted waypoint under the current
  x-progress rule and is not an executed new mission.
- Invoke unchanged terrain-blind nominal construction from the actual query
  state, then the unchanged terrain audit if a nominal exists. Preserve the
  original absolute deadline (9600 ticks); no artificial low-energy state.
- Original queries must reproduce the saved next nominal identity, counts,
  selected proposal and complete audit. Both successful controls must preserve
  their original clear nominal. Repeats compare complete non-timing query files.

Record `no_nominal`, `nominal_clear`, `nominal_terrain_blocked` or
`nominal_other_rejected`. A blocked nominal may legitimately consume only a
prefix before terrain contact; native replay must still agree. A clear audit is
a counterfactual construction/audit result, **not an executed mission landing**.

## Retention, review and stop boundaries

Use create-only outputs beneath the isolated terrain evaluation directory.
Retain source/binary snapshots, exact inputs/specs, prefix proofs, recomputed
clearing proposals, nominal searches/audits, ordered ledgers and a full inventory
receipt. Saved verification launches no probes or new replay. Freeze source and
binary across collection and preserve old captures, the ordinary release binary,
accepted selectors/reports and the currently served rich report site.

Before collection: pure/synthetic tests, review the reconstruction and boundary
logic, and run the maintained developer gate. After: saved verification, a second
read-only evidence review, exact repeats, unchanged-source/protection checks,
and documented results/next design. No delegation is implied by this goal loop.

Stop on runner, proof, source, comparison or bound failures; retain the partial
capture. Finite `no_nominal` is an expected result, not a harness failure. Do not
spend unused allowance on improvised fixes or reruns.

## Decision rule and next phase

1. Earlier states work: prioritize handoff/progress semantics, preserving a real
   obstruction-clearance witness and bounded short-coast safety. Determine
   whether the nominal is clear or simply identifies another obstruction.
2. Only alternatives work: investigate a simple state-based clearing preference,
   not terrain-specific thresholds or a mandatory landing suffix.
3. Neither works: characterize one simple braking/acquisition capability from the
   remaining state/energy pattern; do not expand a large trajectory search.

Mixed results justify a narrow combined explanation, not a universal cure.
Map the smallest implementation plan and its preservation tests from the data.
Actual flight execution, optional 44-case parity, paired 1k/fresh 100 evaluation,
default-cap promotion, report publication/server changes, commits and pushes
remain outside this diagnostic goal and need separate scope.
