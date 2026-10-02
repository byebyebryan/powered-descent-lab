# Waypoint V2 airborne integration results

## Verdict

The [bounded airborne integration plan](waypoint_v2_airborne_integration_plan.md)
reaches its unchanged complete-loop gate on 2026-10-02. Explicit opt-in policy 3
lands all eight uncut clear controls without corrections and all sixteen ordinary
terrain missions, improving ordinary coverage from policy 2's 12/16 to 16/16.
Both corrected-source final runs pass and their 64 per-case summary/flight
payloads match under the existing timing/path exclusions. The
[primary review](waypoint_v2_airborne_integration_acceptance.md) accepts this
supported opt-in checkpoint; no required implementation work remains.

Late review found two guard-only gaps: unexplained audit endpoint mismatch must
be an integrity failure, and only explicitly finite witness statuses may retry.
The primary corrected both without changing commands or numerical policy. The
original matrix allowance was already used, so the user explicitly approved
three additional final checks: one corrected-source policy 2 preservation matrix
and two policy 3 repeats, with unchanged inputs and no tuning. All three pass
their respective preservation/usability checks. Nine focused tests, the fresh
39-row adapter gate and the final 915-test workspace gate also pass. Earlier
full-loop roots remain intact and are not relabeled as corrected-build evidence.

This is an offline, supplied-command simulation planner for the tested vehicle,
Earth gravity, 120/60 Hz, forward route-free transfers and heightfield terrain.
It is not GUI/controller/default promotion, arbitrary-state recovery, swept
collision proof or perturbation robustness. Policy 1 remains the default.

## What changed

Policy 3 preserves canonical ground launch, policy 2 initial intervention spacing
and the entire existing local-clearing search, rank and six-correction cap. Only
airborne nominal regeneration changes. It uses the tested acquisition and
baseline-retaining terminal-time family from actual state and the original
absolute deadline, stopping at the first executable proposal among at most
three ranked physical trials. No numerical family, margin or ranking is tuned.

The shared realizer now separates free-space command construction and independent
replay from research terrain reporting. Runtime never loads a corpus or retained
answer. A new bound complete command program covers acquisition turn/burn,
coast and terminal, with explicit aligned phase boundaries and raw endpoint.
Its terrain audit uses the command consumed by each transition, including an
odd final held pair, and never grants an airborne source-pad exception.

The outer loop remains terrain-blind nominal construction, fixed terrain audit,
local obstruction clearing and replanning from actual handoff. Terrain rejection
does not select another higher nominal arc. A waypoint requires local progress
and its existing continuation certificate, not a landing suffix. Original fuel,
time, attitude/rate and history continue through every segment.

## Complete mission results

| Group | Preserved policy 2 | Policy 3 final A and B | Required |
| --- | ---: | ---: | ---: |
| Clear controls with zero corrections | 8/8 | 8/8 | 8/8 |
| Ridges | 3/4 | 4/4 | At least 2/4 |
| Plateaus | 3/4 | 4/4 | At least 2/4 |
| Successive obstacles | 2/4 | 4/4 | At least 2/4 |
| Sloped obstacle terrain | 4/4 | 4/4 | At least 2/4 |
| All ordinary terrain missions | 12/16 | 16/16 | At least 13/16 |

All sixteen ordinary initial proposals remain genuinely terrain blocked. All
thirty supported complete or partial flights retain integrity and full
source/official replay. Their complete initial cycles, including initial local
selection where applicable, equal historical policy 2 exactly. No scenario,
floor, reserve, deadline or denominator changes.

The four prior ordinary misses now land. Late ridge and late plateau each use
one correction; successive rising uses two, with a new second handoff at H2774;
successive plateaus now lands after its first handoff H2276, instead of needing
the old later H3702 recovery. This distinguishes complete-loop evidence from
merely proving recoveries at four saved states.

The reference plateau still lands with three corrections. Its first E1512 to
H2820 and second E2894 to H3136 are retained; its third now runs E3162 to H3226.
Landing is at H4886, 40.7167 s, with 5189.166947 kg fuel remaining. The three
local trajectory-plus-certificate minima are 21.313368, 6.275370 and 6.549814 m
against the unchanged 5 m reserve. Across all selected local programs, the
smallest such minimum is 6.163038 m on successive rising. These are discrete
deterministic checks, not a tolerance or perturbation guarantee.

Across the full suite, twenty-five actual airborne calls execute twenty-six
physical trials; six selected proposals use powered acquisition. Only successive
plateaus needs a second trial after the existing finite backend shape rejection.
The third-attempt cap remains available rather than being reduced to fit this
sample. The count is not a benchmark of constructor completeness.

## Diagnostic limits

The 700 m high-obstacle diagnostic lands with two corrections; the narrow-target
clear diagnostic lands directly. The 900 m high obstacle, near-source obstacle,
near-target obstacle and long plateau retain NoClearing, stop while still flying,
and preserve complete partial-flight evidence. They are finite-policy misses,
not proofs of physical impossibility. Other gravity and vehicle diagnostics
reject as Unsupported without creating a simulation. Diagnostics have no landing
quota and were not removed from the suite.

Reverse/overshoot recovery, arbitrary powered replanning origins and broader
vehicle/gravity support remain outside this checkpoint. Ground constructor
unification and a dive-and-recover backend are not prerequisites for closure.

## Validation and preservation

The final adapter gate reconstructs all 39 retained airborne states through full
fresh ordinary source prefixes and matches the retained first executable
commands, actual origins and target-plane endpoints exactly. All four designated
former NoNominal handoffs retain terrain, reserve and replay success. It uses
forty trials: thirty selected programs have zero acquisition, nine have powered
acquisition, and one finite rejection falls through. Its 255 source/input
bindings verify before and after and remain current.

The shared-realizer extraction reproduces all 47 retained and eight synthetic
research rows, eight controls, traces, metrics and row identities exactly.
Only header source bindings and artifact identity differ; all 254 final research
bindings remain current. The preliminary reproduction is retained as well.
Of the preceding terminal-time artifact's 252 bindings, 247 remain byte-identical;
only the five intended Rust integration files differ. Existing numerical helper,
canonical launch, old airborne and local-clearing implementations are preserved.

The corrected-source policy 2 matrix matches all 64 historical per-case payloads
under only the existing named timing/path exclusions. Its expected suite exit
is 1 because unchanged 12/16 still misses the 13/16 gate; preservation itself
passes. Historical and current binary/source/runner provenance are recorded as
different builds, not declared identical. No new comparison exclusions apply.
The corrected policy 3 run also matches all 64 pre-hardening physical payloads
under those same exclusions; guard classification changes did not alter any
selected command, handoff or complete/partial trajectory in the frozen suite.

Both policy 3 final matrices use the same binary/source/runner identities and
unchanged frozen suite/input digests. Normal full-root comparison passes on all
64 payloads. Planning latency covers all twenty-four clear/ordinary attempts:

| Measurement | Final A | Final B | Limit |
| --- | ---: | ---: | ---: |
| Planning median | 0.382719 s | 0.383171 s | 2 s |
| Nearest-rank planning p95 | 0.608003 s | 0.614668 s | 5 s |

Development also passes, with median 0.468807 s and p95 0.943213 s, while native
tests were running. Final repeats followed completion of the workspace gate.
Output, execution and replay time are separate; no comparative speedup or
real-time control claim is made.

The corrected source passes 915 workspace tests with four ignored, including
nine focused runtime tests. The ignored archive adapter gate is explicitly rerun
and passes separately. Policy/parser, corpus/tamper, fake-suite, frozen structural,
format and whitespace checks pass. Strict Clippy reports only the unchanged
`route_capability.rs:274` `single_element_loop` baseline lint. All-target workspace
Clippy passes with only that lint allowed and every other warning denied.

Luna owns policy/CLI/suite wiring; the primary owns the coupled realizer,
constructor/audit integration, extra contract checks, physical interpretation
and acceptance. No core, controller, local search, fixture, default, commit,
push or deployment change is included. The pre-existing dirty worktree is retained.

## Evidence and reproduction

Create-only evidence lives beneath
`outputs/research/waypoint_v2_airborne_integration_20261002`:

- `adapter_hardened/summary.json`: corrected-source bound 39-row equivalence gate.
- `adapter_final/summary.json`: preceding 39-row gate before guard hardening.
- `research_hardened/summary.json`: corrected-source bound 55-row shared-realizer
  preservation; only refreshed header bindings and artifact identity differ.
- `research_final/summary.json`: preceding 55-row preservation before guard hardening.
- Four `canary_*` roots: clear, late ridge, successive plateaus and reference.
- `development_policy_3`: the single complete development matrix.
- `preservation_hardened_policy_2`: corrected-source historical-policy preservation.
- `final_hardened_policy_3_a` and `final_hardened_policy_3_b`: accepted corrected-source
  final matrices, with matching complete payloads and provenance.
- `preservation_policy_2`: original preservation matrix before guard hardening.
- `final_policy_3_a` and `final_policy_3_b`: matching complete matrices before
  the late guard correction, not final corrected-build acceptance.
- Earlier `adapter`, `adapter_verified` and `research_preservation` roots remain
  intact; review added endpoint/phase checks before final source was frozen.

The three corrected-source matrices share these recorded identities:

| Bound item | SHA-256 |
| --- | --- |
| Release CLI binary | `e3d28831c46efbc18f6bec796b7db735b509f59116db041d1636417efc09baba` |
| Rust source tree | `41746d27391e125daccda6b64b755c774456c35f3dfbfbf305c4a5a5d71c38b6` |
| Suite runner | `3dfff648d36c509c353a363593ead5d2f8b9d177e8955f406ab60cf84ea2321b` |
| Frozen suite fixture | `92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c` |
| Expanded scenario inputs | `c340bb444a24f6c99197ffd18e4f38460c17e80c5e46f03bb4a49cb554f8c5a5` |

Bindings verify before/after each run and remain current after documentation
closure. The extra three matrices are an explicit user-approved guard-validation
extension, not uncounted development runs or a reset numerical-policy budget.

Reproduce into a new root; never overwrite a retained run:

```sh
rtk cargo build -p pd-eval --release
rtk proxy node scripts/run_waypoint_v2_practical_suite.mjs \
  --bin target/release/pd-eval --policy-version 3 --output-dir NEW_ROOT
rtk proxy node scripts/run_waypoint_v2_practical_suite.mjs \
  --compare FINAL_ROOT_A FINAL_ROOT_B
```

The first usable supported opt-in V2 checkpoint is accepted on the corrected
build. Remaining diagnostics are recorded limits, not automatic next research.
Any GUI/controller/default adoption or broader scope needs a separate decision
and its matching validation. Review and commit may follow only when requested;
neither occurs as part of this checkpoint.
