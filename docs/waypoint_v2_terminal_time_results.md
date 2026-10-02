# Waypoint V2 terminal time construction results

## Verdict

The approved [bounded plan](waypoint_v2_terminal_time_plan.md) completes on
2026-10-02 with a positive constructor result. The same research acquisition
family now lands all eight uncut flat/uphill/downhill controls, within the
unchanged 0.925 acquisition and terminal thrust budget, with phase-specific
body reserve and independent replay. Its previous ground result was 2/8.
No launch extension, extra acquisition seeds or margin relaxation was needed.

All 39 retained airborne inputs keep free-space landing witnesses. The four
former NoNominal handoffs retain actual-terrain landing, body reserve and replay.
This supports the shared finite-acquisition approach on these inputs; it is not
a universal feasibility envelope or full repeated-handoff planner acceptance.
The [separate primary review](waypoint_v2_terminal_time_review.md) accepts this
research checkpoint and recommends a separately planned opt-in integration.

## What changed

The new `waypoint-v2-terminal-time-v1` identity keeps an admissible baseline
duration unchanged. After a finite rejection at an eligible entry, it tries at
most two derived horizontal shape times: zero initial lateral acceleration and
affine lateral acceleration. It rounds to even ticks, clips to discrete
no-reversal and original-deadline limits, then uses the unchanged reference
builder and safety predicates. Original successful durations are comparison
controls only, never candidate inputs. Interior terrain cannot choose the time.

Ground preparation, acquisition seeds, turn/burn estimates, four coast identities,
outer ranking, physical realization and landing predicates are unchanged. The
adapter uses the old acquisition evaluator once and reads its cached plan;
it does not add another acquisition solve. Maximums remain thirteen seeds,
three turn updates, three durations per coast and three physical attempts per row.
There are no shadow exceptions, snapshot restores, fuel resets or clock resets.

## Proven-entry gate

Fresh original full programs reproduce actual S/T, final states and raw contacts
exactly. At each actual T, the unchanged vertical braking time rejects lateral
reversal, the first alternative rejects coupled thrust, and the affine alternative
passes all existing analytical checks under the original deadline and budget.

| Actual entry | Vertical baseline s | Selected affine time s |
| --- | ---: | ---: |
| Clear 685 | 18.383 | 12.983 |
| Clear 735 | 19.350 | 13.500 |
| Clear 845 | 30.617 | 16.150 |
| Clear 915 | 41.783 | 17.750 |
| Uphill 845 | 27.033 | 14.650 |
| Uphill 915 | 20.633 | 14.000 |
| Downhill 845 | 24.700 | 16.150 |
| Downhill 915 | 20.700 | 15.500 |

This is an analytical gate, not alternative flight execution at the old T.
The broad study below separately establishes new acquisition, attitude alignment
and terminal commands from live state. No instant attitude change is assumed.

## Actual new ground flights

Every first valid proposal lands with stable raw target contact before tick 9600.
Source preparation independently replays and has no recorded reserve violation.
All three attempted proposals per ground row land; coverage is still eight cases,
not twenty-four independent controls. The reported first proposal is selected by
the frozen analytical/executable order, never by a later terrain-success result.

| Case | Acquisition peak thrust fraction | Terminal peak thrust fraction | Target contact tick |
| --- | ---: | ---: | ---: |
| Clear 685 | 0.837515 | 0.786690 | 2906 |
| Clear 735 | 0.840031 | 0.784273 | 3000 |
| Clear 845 | 0.847744 | 0.775551 | 3210 |
| Clear 915 | 0.851239 | 0.770782 | 3332 |
| Uphill 845 | 0.883458 | 0.741792 | 3210 |
| Uphill 915 | 0.883056 | 0.737025 | 3336 |
| Downhill 845 | 0.812883 | 0.892717 | 3512 |
| Downhill 915 | 0.818985 | 0.889340 | 3680 |

Applied demand uses command propagation with changing mass. Full-thrust upright
preparation remains a separately governed source phase, not terminal demand.
Body reserve uses the existing phase-specific source/terminal contact-corridor
exceptions; it does not mean five metres of clearance through touchdown.

The first seed in each ground row is the existing `upward_shaping` choice with
burn fraction 0.45 and coast entry 2. All seed/acquisition fields and ground
preparation exactly match the old ledgers. Only terminal freedom changed.
The flat/uphill successes no longer require the approximately 0.993 shadow
terminal throttle disclosed in the preceding diagnostic.

## Airborne preservation and remaining limitations

All 39 inputs are reconstructed through their complete original ordinary prefixes.
Full snapshots agree exactly, including clock, fuel, attitude and mission history;
maximum float delta is zero. No retained state is restored into an executor.

| Population | Free-space witness rows | First valid proposal terrain landings | Terrain landings with body reserve |
| --- | ---: | ---: | ---: |
| Clear ground | 8/8 | 8/8 | 8/8 |
| Historical captures | 12/12 | 12/12 | 12/12 |
| Local handoffs | 27/27 | 20/27 | 18/27 |
| Separately labeled synthetic conditions | 7/8 | 6/8 | 6/8 |

Each airborne row's first valid proposal retains its old terrain and reserve
classification, not just aggregate counts. Seven local proposals remain terrain
blocked; two others land but violate noncontact body reserve. Those are still
rejections requiring another local piece, not permission to loft the nominal
route around terrain. Handoff-row counts do not establish mission success.

The four former NoNominal recoveries retain their first valid seed and finish:
plateau-late H2446 to H4062, ridge-late H2426 to H4080, successive-plateaus H3702
to H5476, and successive-rising H2804 to H4656. Four historical near-apex rows
and plateau-wide H3662 now select zero acquisition rather than upward shaping;
three other local rows select an earlier coast entry. All eight changed airborne
first proposals land with reserve. Adequate existing motion need not be forced
onto the previous powered profile.

Two newly admitted analytical trials encounter the unchanged backend's
"at most one future apex, not dive and recover" rejection: successive-plateaus
H2276 and synthetic steeper-arrival. Both find a valid proposal within the
original three-attempt cap. These are recorded finite failures, not emitted valid
proposals, terrain regressions or physical impossibility. Their body/thrust
metrics are null because no corresponding flight was evaluated. Counting the
first trial as the first valid proposal would incorrectly imply a regression.

Synthetic low-fuel remains a finite construction miss; synthetic uphill-target
has a free-space witness but remains terrain blocked. The other six synthetic
first valid proposals land with reserve. These classifications match the old
study. Near-zero lateral speed remains baseline-only, and the new small family
can still miss states outside these inputs. No universal coverage is claimed.

## Evidence, cost and validation

Accepted final-source roots are
`outputs/research/waypoint_v2_terminal_time_20261002/final_a` and `final_b`.
Complete summaries and sorted bindings are byte-identical; repeats are
reproducibility, not additional independent cases. All 252 bindings verify
before, after and independently from disk.

- Summary SHA-256: `9570853b4a44c8a2d758698bfcabe4a5fcf467db04fe636e9e62375c3a3de2ae`.
- Bindings SHA-256: `d765ff25994c9c8f88d1f031bcddadc00ff85a738ba6e52a3c6935dc495a32f7`.
- Identity: `fnv1a64:c2d79d3fe9149f18`.

Across 55 study rows, all 664 acquisition seed records and 1640 coast identities
match the prior study. All 319 admissible baseline screens remain identical;
139 additional entries become admissible. Candidate screens increase from
1640 to 3890 and actual analytical reference builds from 1395 to 3645, excluding
the matched-control gate. The largest row has 130 candidate screens, below the
156 cap. Total physical attempts rise from 133 to 154 as more entries qualify;
no row exceeds three. Faster runtime planning is not established by this pass.

Separate `timings.json` files record nondeterministic research wall time. Median
row time is 0.240–0.242 s, p95 0.389–0.408 s, maximum 0.441–0.443 s; summed
row work is 13.59–13.91 s per run. These include prefix reconstruction, up to
three physical realizations and audit/replay work on this local debug build;
they are not isolated runtime constructor latency or byte-identical evidence.
`dev_a` is retained development evidence, not the accepted final source.

Final-source workspace tests pass 904 with three intentionally ignored.
Fifteen focused timing/CLI tests pass, including nine delegated pure-helper
tests and five primary integration/reporting tests. The two archive-dependent
ground diagnostic tests also pass when explicitly requested. Corpus/tamper,
fake-suite, frozen 8/16/8 structural/input, formatting and whitespace checks pass.
Structural checks do not measure new mission acceptance.

Strict Clippy still fails only on the unchanged `route_capability.rs:274`
`single_element_loop` baseline lint under rustc 1.99.0. All-target workspace
Clippy passes with only that lint allowed and all other warnings denied.
No unrelated file is changed to silence it. All 68 fixture files, 1504 monitored
retained artifact files, 204 protected source/script/input files and the existing
51 characterization plus 25 terminal helper bodies retain their hashes.

Reproduce from the repository root with a new create-only output directory:

```sh
rtk cargo run -p pd-eval -- waypoint-v2-terminal-time \
  --corpus outputs/research/waypoint_v2_nominal_characterization_20261001/corpus.json \
  --output-dir outputs/research/waypoint_v2_terminal_time_20261002/NEW_ROOT
```

Luna owns the pure duration helper and its tests; the primary owns integration,
physical interpretation, additional contract tests and the separate acceptance
review. Runtime policies 1/2, local planning, terrain, controller, contact physics
and defaults remain unchanged. No commit, push or deployment is included.
