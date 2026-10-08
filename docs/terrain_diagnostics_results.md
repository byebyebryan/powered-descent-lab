# Procedural-terrain diagnostic pack results

Date: 2026-10-07 (local). Completes the
[bounded selection/build/validation plan](terrain_diagnostics_plan.md).
The allowance is closed: **13/13 measured attempts**, no retry or replacement.
No planner, controller, simulation or native evaluator behavior changed.

## Verdict

The ten-world diagnostic pack is built, reviewed and validated. All ten primaries
reproduce their complete original 1k flight records, excluding only the three
declared wall timings. Three fixed repeats reproduce their new primaries by the
same rule. Four successful comparisons retain verified landings; six expected
finite failures retain their original honest outcomes. This is successful
diagnostic validation, **not improved planner coverage**.

The missions now give concrete tests for three capabilities: escaping rising
terrain close to departure, establishing a new nominal after a fast late H, and
gaining useful distance across successive corrections. These are observed
failure families, not three proven physical root causes or evidence that one
fix will solve all 267 misses. The inspected 1k population is development data.

## Frozen missions and actual results

| Family | Mission | Recipe | Preserved result | Corrections |
| --- | --- | --- | --- | ---: |
| Departure | `random-327` | Mountains 8x | `NoClearing` before departure | 0 |
| Departure | `random-791` | Successive ridges 8x | `NoClearing` before departure | 0 |
| Departure comparison | `random-258` | Mountains 8x | Verified target landing | 3 |
| Airborne acquisition | `random-024` | Mountains 4x | `NoNominal` at actual H | 1 |
| Airborne acquisition | `random-516` | Broad massifs 8x | `NoNominal` at actual H | 1 |
| Acquisition comparison | `random-271` | Mountains 8x | Verified target landing | 4 |
| Useful progress | `random-030` | Mountains 4x | `CorrectionLimit` | 6 |
| Useful progress | `random-280` | Mountains 8x | `CorrectionLimit` | 6 |
| Progress comparison | `random-253` | Mountains 8x | Verified target landing | 6 |
| Direct preservation | `random-000` | Mountains 4x | Verified direct target landing | 0 |

Repeats of `327`, `024`, `030` reproduce `NoClearing`, `NoNominal` and
`CorrectionLimit` respectively. They are outside the ten-primary denominator.
Every recorded attempt passes integrity and original-source replay. There are
no crashes, fuel stops, timeouts, collection failures or comparison exceptions.
The six primary finite stops remain physically `flying`, mission `in_progress`;
neither exit zero nor fingerprint equality turns them into landings.

## What the comparisons teach us

### Departure: clearance and forward progress are different demands

The first nominal conflicts are at x=20.149 m (`327`) and x=22.123 m (`791`),
inside the 42 m source pad preparation region. Each final local search has
eight enumerated entry clocks and 126 propagated query rows. Neither produces
a supported forward-progress H, so ranking different accepted candidates
cannot help: there are none.

Comparison `258` also encounters terrain near the source, at x=41.753 m. Its
first H reaches x=54.656 m, later corrections reach x=157.798 and 770.113 m,
and it lands. This is similar operating context, not identical geometry. The
diagnostic question is whether a simple maneuver can establish clearance and
then useful forward escape where the current fixed templates cannot. It does
not prove either failing world physically impossible or guarantee that a
lift-then-advance maneuver would solve it.

### Acquisition: a positive cheap braking estimate is not a recovery certificate

| Mission | Actual last H x | Forward speed | Height above target surface | Cheap braking room | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| `024` | 905.733 m | 83.056 m/s | 364.186 m | +0.057 m | All analytical candidates rejected |
| `516` | 904.726 m | 82.700 m/s | 486.337 m | +3.013 m | All analytical candidates rejected |
| Comparison `271` | 1000.524 m | 63.576 m/s | 517.640 m | +21.289 m | New nominal and target landing |

Both failures have **zero new nominal physical witness attempts**. Their
analytical admission rejects acquisition demand, forward-reversal states and/or
terminal requirements before execution. Each still has more than 54 s of the
original budget and more than 5600 kg fuel at H. They did not run out of fuel or
time, and they never reached a failing landing-controller execution.

We must distinguish an overly restrictive nominal family from an H requiring
too much change over too little distance. The examples do not resolve that
causal choice. A future pass can test one simple braking/acquisition capability
or an upstream less-demanding H without requiring an entire landing suffix at
each local candidate. Simply increasing the positive-room threshold would
reintroduce tuning without proving the missing capability.

### Progress: correction count alone is the wrong target

After six corrections, `030` is at x=385.869 m and `280` at x=384.319 m: roughly
815 m remain. Both still have about 42 s and 5290 kg fuel. Their post-first-H
increments are respectively 56/25/26/37/40 m and 23/28/31/30/37 m. In `280`,
four intervening nominal conflicts occur less than two seconds after H.

Comparison `253` also uses six corrections, reaches its last H at x=762.972 m,
and then lands. It includes one reblocking interval below two seconds. Therefore
neither banning every quick reblock nor demanding fewer than six corrections
is a valid simple success criterion. Test useful geometric/state progress and
the resumed maneuver; do not just raise the cap or call every short hop wrong.
Recorded x increments and conflict delays describe different worlds, not a
controlled experiment establishing a single defective terrain feature.

Subsequent clarification: the [bounded cap-only probe](terrain_cap_probe_results.md)
preserves both original H6 prefixes. `030` then lands with seven corrections;
`280` reaches a later `NoClearing` after eight. Thus these original unfinished
flights are not themselves proof of defective short-hop progress. The table
above remains the authentic cap-6 characterization, not the later probe result.

## Implementation and validation

- [Ten checked-in scenarios](../fixtures/research/terrain_diagnostics_v1/README.md)
  are byte-identical to authenticated original 1k inputs. Their original IDs,
  seeds, metadata, pads and terrain remain unchanged. No generator, manufactured
  scenario or 5.1 GiB local survey is needed to use these inputs.
- The [frozen contract](../studies/terrain_profiles/diagnostics_plan.json) retains
  source manifest/receipt identities, raw original flight digests, complete
  non-timing fingerprints, expected result tuples and compact observations.
- The [collector/verifier](../studies/terrain_profiles/diagnostics.py) reuses
  native preflight/flight execution, existing evidence helpers and common rich
  detail output. It snapshots source/inputs, predeclares all attempts, has finite
  case/collection bounds, retains partial stops and never retries or publishes.
- The default baseline mode enforces exact characterization. Explicit future
  `--candidate` mode permits repaired failure trajectories while requiring all
  four comparison landings, direct-control preservation and exact repeats.
  Candidate mode was exercised only by synthetic unit tests, not a new flight
  campaign in this pass.
- All **74 terrain-study tests** pass, including **17 diagnostic tests** for
  selection, input/hash/seed identity, family predicates, H versus certificate
  end, landing tuple, candidate comparisons, source/receipt/ledger binding,
  create-only roots, partial stops without retries, and read-only verification.
- All **11 maintained developer checks** pass, including Rust workspace/CLI
  tests, formatting, strict Clippy and 52 Node tests. The optional 44-case saved
  numerical parity campaign was not invoked. Closure docs are checked separately.
- Collector final verification and a separate saved Python verification pass.
  An independent read-only Node comparison also matches all 13 complete flights
  against the original baseline, excluding only wall timings.
- All 13 rich detail pages have exact waypoint annotation bindings: **34 actual
  H markers** across primaries/repeats. Existing trajectory, velocity/thrust,
  hovered samples, event/config, landing/flight/controller statistics and mission
  data remain in the common template. These are static evidence/payload checks,
  not browser or human visual acceptance and not report-site publication.
- All **146 selected source/input files** are unchanged across collection and
  match the frozen snapshot. The evaluator remains the original 1k executable.
  All **48 protected accepted selector/report files** are hash-identical.

The [evaluation workflow](evaluation.md#procedural-terrain-diagnostic-pack)
contains maintained collection and read-only verification commands. The default
44-case planner pack, accepted report site and server lifecycle are unchanged.
Existing unrelated/uncommitted 1k evaluation plumbing remains preserved.

## Retained evidence

Capture:
`outputs/eval/planner_v2_random_terrain/capture-diagnostics-20261007-v1`.
It retains 261 receipted files, about 98 MiB including frozen source/inputs,
preflights, ordered ledger, full native flights/summary/scenario artifacts,
common rich detail reports and compact diagnostic results. Measured attempt
wall times total 20.595 s; this is an observed collection cost, not a controlled
performance benchmark or per-tick game-loop guarantee.

- Evaluator SHA-256:
  `fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af`.
- Diagnostic contract SHA-256:
  `d96ba90ee2263d91f3ed92e6fc543fcf44b4f18e2a1c31ab5c456a23081a913a`.
- Capture manifest SHA-256:
  `1372da0120b7dbb9a2eb72ae84c2ebc6a8dbbdf823c6db9ddd95b50a9b814a7c`.
- Receipt SHA-256:
  `03b8ac8f74b7fd7290bf0e65cd1e306fbe47da27689631e234f881d1eb6ea09c`.
- Recorded Git base: `41506b8`; the dirty-source/input inventory binds the
  actual diagnostic tooling, not HEAD alone.

## Recommended next decision

Start with the clearest bounded mechanism: departure clearing. Investigate one
simple clearance-then-forward maneuver using `327/791`, preserve `258` and the
direct control, and use the other diagnostics to reveal downstream regressions.
This is a candidate direction, not a validated fix or authorization to execute
another pass. Do not reopen terrain-aware direct-arc selection, expand a large
fixed grid or build an end-to-end recovery solver.

If that capability helps, test it across the relevant development failures and
then on a separately frozen untouched sample before claiming generalization.
The mixed continuation-admission bucket is deliberately deferred; these ten
missions do not cover every `NoClearing` cause or establish a broad expected
percentage gain. No commit, push or publication occurred, and no flight beyond
the 13-attempt allowance was executed.
