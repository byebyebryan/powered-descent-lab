# Practical waypoint V2 implementation results

## Verdict

The complete experimental V2 loop is implemented and repeated on the frozen
32-case suite. It follows terrain-blind nominal generation, fixed terrain audit,
locally ranked corrections and replanning from actual handoffs. Uncut flat,
uphill and downhill controls all land directly. The old reference plateau now
lands through three corrections, preserving its original first handoff.

The one allowed initial-entry revision improves ordinary coverage from 8/16 to
12/16. That is 75 percent, with every family meeting its floor, but it is still
one landing short of the declared 13/16 usable-planner gate. Do not call the
goal achieved or silently lower that bar. Four ordinary missions stop planning
while still flying, with NoNominal after a proven clearing; none crashes on the
active branch.
Further policy work requires another decision.

This is a supplied-command, offline simulation planner for the current vehicle,
Earth gravity and 120/60 Hz forward route-free LandingOnPad setup. It is not
legacy transfer-controller tracking, arbitrary-state viability, swept collision
proof, real-world autonomy, or production/default promotion.

## Frozen suite results

| Group | Policy 1 | Policy 2 | Required |
| --- | ---: | ---: | ---: |
| Clear controls with no correction | 8/8 | 8/8 | 8/8 |
| Ridges | 2/4 | 3/4 | At least 2/4 |
| Plateaus | 2/4 | 3/4 | At least 2/4 |
| Successive obstacles | 0/4 | 2/4 | At least 2/4 |
| Sloped obstacle terrain | 4/4 | 4/4 | At least 2/4 |
| All ordinary obstacles | 8/16 | 12/16 | At least 13/16 |

All sixteen ordinary initial nominal proposals are genuinely blocked in both
policies. No case, terrain, floor, reserve, deadline or denominator changes.
The [revision record](waypoint_v2_practical_policy_revision.md) freezes the
single uniform entry-spacing change before its measurements. Policy 1 remains
the API/CLI default; policy 2 is explicit. Later entry formulas, local grid and
ranking remain unchanged.

Policy 2 diagnostics land the 700 m high-obstacle case with two corrections and
the narrow-target clear case directly. The 900 m high obstacle, near-source,
near-target and long plateau exhaust clearing. Other gravity and vehicle inputs
reject as Unsupported without creating a simulation. Diagnostics have no
landing quota; every outcome retains honest typed evidence.

## Reference flight and continuity

The first correction remains source entry 1512 to H 2820, at
(-337.771260, 382.820590) m and (38.313173, -10.902736) m/s, with fuel
5682.279573 kg and an idle upright held command. Two later corrections execute
from E 2894 to H 3136 and E 3174 to H 3236. Every H is the retained actual
flight, not its separate two-second certificate endpoint or a restored snapshot.
No landing requirement or terrain far edge enters local row selection.

The reference lands at tick 4574, 38.1167 s, with 5252.905673 kg fuel remaining
and 1047.094327 kg consumed. Its three local trajectory-plus-certificate minima
are 21.313368, 6.655374 and 5.043369 m against the unchanged 5 m reserve. The
last has only 0.043369 m reserve surplus: this discrete deterministic evidence
does not establish perturbation robustness. Ordinary whole-flight clearance
summaries include launch and terminal contact and therefore are not the local
5 m acceptance metric.

Every supported complete or partial flight agrees with independent whole-source
execution and official bounded action replay on full state, raw incoming contact,
actions, events and sample cadence. Only initial launch may use the original
source-pad exception; local segments require full reserve and later nominal
descent uses its existing terminal corridor. The active branch never consumes
a rejected nominal suffix or the continuation certificate.

## Remaining coverage gap

| Ordinary miss | H tick | Target distance m | Horizontal speed m/s | Height m |
| --- | ---: | ---: | ---: | ---: |
| Late ridge | 2426 | 288.850 | 76.949 | 318.285 |
| Late plateau | 2446 | 276.025 | 76.949 | 319.951 |
| Successive rising ridges | 2804 | 288.864 | 64.033 | 363.766 |
| Successive plateaus | 3702 | 276.221 | 69.631 | 321.815 |

All 56 airborne nominal rows reject at each of these actual states. Across the
224 attempts, 216 reject with the paired-throttle maximum/remaining-mass check
and eight reject the single-future-apex shape rule. Their live vehicle masses
are far above the two-tick fuel-burn bound, so the combined throttle error is
an acceleration-demand limit, not an unsupported dry-mass configuration.

The measured gap is therefore finite nominal-family coverage after locally
safe clearing, not a renewed floor-cutaway or first-step crash problem. High
horizontal speed close to the target is a plausible contributor; no optimal
recovery or physical impossibility claim follows from the finite rejections.

Following the user's clarified target-arc semantics, the next priority is the
[unified nominal constructor](waypoint_v2_unified_nominal_plan.md), before tuning
local ranking. It should preserve acceptable vertical motion and estimate finite
acquisition from current state, rather than require an exact minimum reference
profile. The follow-up starts with estimator/entry characterization and a
separate review; no improved flight or recovery is established here. Keep local
acceptance separate from immediate landing and preserve these runs and the same
matrix. The [goal amendment](waypoint_v2_goal_amendment.md) does not grant another
automatic revision of this measured policy.

## Timing and repeat evidence

Two final policy 2 runs each measure all 24 clear/ordinary attempts, including
failures. Planning median is 0.3981/0.3976 s and nearest-rank p95 is
0.6803/0.6702 s, below the 2/5 s goals. Planning includes generation, live-origin
queries, fixed audits, local search and selected validation. Active execution
is at most 0.0012 s and independent replay at most 0.057 s per common attempt;
artifact writing is separate. Maximum common total wait is 0.929/0.913 s.
Compilation, preservation and eight diagnostics are outside this denominator.
No real-time guarantee is inferred.

Repeat comparison passes all 64 complete per-case summary/flight JSON payloads,
excluding only the named timing/path fields. Decisions, supplied commands,
states, contacts, local certificates, and replay flags remain compared. Both
runs bind the same binary, source tree and runner, unchanged during each run.
The binary/source relationship is additionally checked by the completed release
build; hashes alone would not prove that relationship.

Retained local roots under `outputs/research/waypoint_v2_practical_20261001`:

- `development_policy_1_a`: partial odd-contact adapter failure, retained.
- `development_policy_1_b`: complete first policy matrix, 8/16 ordinary.
- `final_policy_2_a` and `final_policy_2_b`: repeated revised matrix, 12/16.
- `final_repeat.json`: named-exclusion comparison receipt.
- `preservation_final`: fresh sealed canary and earlier physical controls.

The expanded-input SHA-256 stays
`c340bb444a24f6c99197ffd18e4f38460c17e80c5e46f03bb4a49cb554f8c5a5`.
Final binary SHA-256 is
`5fb0aacf67bbc4b773e84e7220250c40d534d8666a1153ced9fc38f5c1bbb5f6`;
the suite source-tree SHA-256 is
`8cfd2f76c0a1b39a0cb3d7582e42659d5ce175b2863e46eb0f19061a658ba096`.

## Validation and usage

Final workspace validation passes 872 tests, with one existing ignored test and
no failures. Strict workspace Clippy, formatting, whitespace, focused
pure-policy/runtime/odd-contact/tampering/CLI/create-only checks, exact-input
expansion and the fake-CLI harness checks also pass.

The fresh historical-preservation run passes all five sealed canary gates. Its
physical experiment is byte-identical to the accepted earlier experiment;
eight canonical flights, twelve airborne continuations and twenty-four
source-rest controls preserve their compared physical evidence under the old
named observational exclusions. The expanded source binding covers 146 files
and remains unchanged during the run. This is one fresh preservation run
compared with historical evidence, not a second new canary-repeat claim.

Run an explicit revised flight with a fresh output root:

```sh
rtk proxy target/release/pd-eval waypoint-v2-flight --policy-version 2 \
  --scenario SCENARIO.json --source-pad-id pad_source --target-pad-id pad_main \
  --output-dir NEW_ROOT
```

Use `--preflight-only` instead of `--output-dir` for an input-only check. Normal
execution writes scenario, compact summary, full flight ledger and an ordinary
trajectory report; it does not run the preservation canaries per mission.
To repeat the matrix or compare complete roots, use
`scripts/run_waypoint_v2_practical_suite.mjs --help`.

Primary owns loop, contracts, guard/query integration, measured revision and
acceptance; Luna owns CLI, create-only reporting, suite and fake-harness checks.
No core integrator, contact/landing predicate, legacy controller, production V1
dispatch, default promotion, commit, push or deployment changes.
