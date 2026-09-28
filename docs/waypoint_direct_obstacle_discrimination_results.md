# Frozen direct-generator obstacle discrimination results

This is opt-in evaluator evidence, not a change to `pd_plan::plan()` or a
waypoint executor. The [protocol](waypoint_direct_obstacle_discrimination_protocol.md)
separates exact fixed-command terrain twins from independently regenerated
complete direct witnesses. Generator policy, vehicle, physics/contact rules,
source fitter, clearance rules and accepted-only ranking were not changed.

Development and the sealed eight-case gate passed without retuning: six cases
have complete accepted direct witnesses; two high-obstacle cases are valid
finite `Unknown`. The 900 m late/broad case blocks the selected flat flight
but accepts a different direct arc. This closes the bounded discrimination
question, not waypoint necessity or production integration.

The follow-up [terminal diagnostic](waypoint_direct_terminal_admissibility_results.md)
now isolates ideal first-contact geometry from matched-entry execution cadence.
The original gate and its witness ledger below remain unchanged.

## Preservation and development

All six earlier 600/1,000 m flat/uphill/downhill generation artifacts were
independently regenerated after shared log/clearance-helper extraction and
are byte-identical to the original retained artifacts. The exact known-flat
development summary also retains SHA-256
`3e479a9e46c1753c0950076f2434c607db7588003830762c45298fc3e3953e35`.

The four development controls passed every declared gate, without changing
terrain or tuning the generator. Two fresh output paths have byte-identical
gate summaries and byte-identical full generation artifacts for all four
cases. Gate identity: `fnv1a64:589a3db80a23d2a0`; raw summary SHA-256:
`b51082ce9dcdf45a5ba4b818d81fee62608ca9241360e8af6cacfafc049a7709`.

| Development control | Regenerated result | Scheduled / accepted | Fixed flat program |
| --- | --- | ---: | --- |
| Uncut flat, 800 m span | Direct | 9 / 4 | Stable target landing |
| Centered 160 m obstacle | Direct | 9 / 4 | Safe overflight and stable target landing |
| Historical 440 m boundary obstacle | Unknown | 0 / 0 | Source-bridge terrain crash, tick 1541 |
| Historical 1,600 m obstacle | Unknown | 0 / 0 | Source-bridge terrain crash, tick 1562 |

Each case retains all twenty rows. On the 440 m obstacle, the fixed program
first loses the declared 5 m actual-body reserve at tick 1538 (clearance
`3.066679 m`) and contacts terrain at tick 1541 (`12.841667 s`). On the
1,600 m obstacle it loses reserve at tick 1560 (`2.015678 m`) and contacts at
tick 1562 (`13.016667 s`). Both occur during the source bridge, not on the
first physics step or at target touchdown. Requested commands, motion and
fuel match the flat replay exactly through the contact tick; ordinary and
neutral contact replay agree under the existing stable-touchdown convention.

The unknown controls fail the finite analytical seed family before fitting
full schedules. Their retained reasons include terrain clearance, source
bridge/attitude, coupled thrust, terminal bridge/attitude, and, for one 440 m
seed, mission time. They are terrain-associated mixed bounded rejections,
not proof that terrain is the sole cause or that every direct transfer is
physically impossible. No new waypoint execution result is claimed.

## Frozen fresh gate

Development was accepted after 703 workspace tests in twelve suites, strict
all-target workspace Clippy, formatting, and diff checks. Both code-freeze
summaries are identical: freeze `fnv1a64:57b8caadc1937c47`, production inputs
`fnv1a64:06c560a38ef95e02`. The fresh manifest retains its pre-implementation
SHA-256 `647df7bc94d02a9b48b773c45159e0ffa15bfca232a13b4e21164e2251a8f5b4`.

All eight cases were evaluated in their sealed order under that one freeze,
with no errors or omitted rows. The family retains 160 rows: 84 analytical
skips, 76 fitted/replayed schedules, 34 accepted witnesses, and 42 rejected
target-contact crashes. Both gate summaries and all eight full case artifacts
are byte-identical across fresh paths. Gate identity:
`fnv1a64:a484e83d323f3e02`; raw summary SHA-256:
`3282a7e533809009a9e5cf377ad8cbbfea598a2b4840df533fbe696015409baa`.

| Span / profile / height | Regenerated result | Scheduled / accepted | Fixed flat program |
| --- | --- | ---: | --- |
| 700 m / flat / 0 m | Direct | 12 / 4 | Safe; stable target contact |
| 700 m / low / 140 m | Direct | 12 / 4 | Safe overflight |
| 700 m / high / 385 m | Unknown | 4 / 0 | Coast terrain crash, tick 2425 |
| 700 m / late broad / 245 m | Direct | 9 / 4 | Safe overflight |
| 900 m / flat / 0 m | Direct | 14 / 9 | Safe; stable target contact |
| 900 m / low / 180 m | Direct | 14 / 9 | Safe overflight |
| 900 m / high / 495 m | Unknown | 2 / 0 | Coast terrain crash, tick 2199 |
| 900 m / late broad / 315 m | Direct | 9 / 4 | Coast terrain crash, tick 2468; different direct accepted |

The high-700 twin first loses the 5 m reserve at coast tick 2375 and contacts
at 2425 (`20.208333 s`). High-900 loses reserve at tick 2196 and contacts at
2199 (`18.325 s`). Late/broad-900 loses reserve at tick 2445 and contacts at
2468 (`20.566667 s`). All counterfactuals preserve exact commands, motion and
fuel through first terrain-driven contact; every ordinary/neutral replay
parity check passes. Unblocked twins retain stable target contact.

The late/broad-900 result demonstrates the important distinction: the flat
winner's multiplier `1.25`, offset `-240`, planned `37.10 s` program is
blocked. Regeneration instead accepts multiplier `1.5`, offset `-180`, planned
`43.35 s`, stable target contact at `43.283333 s`, with `1,158.805564 kg` fuel
used. Its source handoff is higher (`419.42 m` rather than `309.31 m`), and
its complete body-clearance and contact gates pass. No waypoint was added.

The 700 m flat, low and late/broad winners each plan `38.60 s` and contact at
`38.525 s`; the 900 m flat/low winners plan `37.10 s` and contact at `37.0 s`.
Selected hull-penetration margins are `+9.141 mm` for the 700 m winners,
`+2.647 mm` for 900 m flat/low, and `+2.385 mm` for the alternative 900 m
late/broad winner. The 900 m flat/low maximum-foot-clearance margin is also
only `+2.758 mm`. These are current core predicate margins at nominal first
contact, not robustness certificates or continuous swept-path guarantees.

## What the high-case unknowns mean

The two high cases are not merely “all arcs hit the obstacle.” Regeneration
finds four complete schedules at 700 m and two at 900 m that clear terrain,
pass strict source handoff, descend into the terminal bridge, and fail only
the complete acceptance gate `first_contact_stable_safe_on_target`. All six
first contact at the target during the terminal bridge, not on launch.

Their actual contact predicates fail both maximum-foot clearance and normal
closing speed: at 700 m the upper foot is about `0.43024 m` above terrain
against the `0.15 m` limit, with normal speed `3.05117 m/s` against `3 m/s`;
at 900 m the upper foot is about `0.41288 m`, with normal speed `3.33437 m/s`.
Hull penetration, tangential speed, attitude limit and angular-rate predicates
pass. Source position errors are below `5.73e-8 m`, and velocity errors below
`1.08e-9 m/s`, against the unchanged `1e-6` tolerances.

The four 700 m contacts occur at `63.425–64.925 s`, about `1.6167 s` before
their planned profile ends; the two 900 m contacts occur at
`67.191667–67.691667 s`, about `1.55 s` before their planned ends. The generic
generator therefore retains a real landing/contact limitation on these
terrain-compatible candidate programs. This is not a measured causal
isolation of terminal cadence versus reference geometry, and it does not
justify changing core thresholds or declaring a waypoint physically required.

## Verdict and next decision

Keep three findings: safe terrain overflight remains Direct; a blocked chosen
arc can still have another accepted Direct arc; and finite Unknown can arise
after successful obstacle clearance because landing has not been achieved.
The original flat/uphill/downhill cutaway mismatch remains closed in the
opt-in nominal primitive, while production V1 still uses its unchanged chord
model. No new source-first-step crash was encountered.

Stop this frozen pass here. The most informative next bounded research
question is terminal capture at actual first core contact for the six rejected
high-case schedules, with the launch/source prefix and terrain fixed. Compare
reference contact geometry and execution using matched terminal-entry states;
do not restart source fitting or turn these unknowns straight into waypoint
demand. Any terminal-policy change needs a separately approved pass and new
held-out inputs, preserving the accepted flat/low/alternative-direct controls.
Waypoint-state composition remains a separate unimplemented capability, and
robust contact remains necessary before production/default promotion.

The primary owned sealed inputs, shared replay, CLI integration, gate review,
physical evaluations and acceptance; Luna owned the bounded runner and pure
ledger/refusal tests. Final code validation passed 703 workspace tests in
twelve suites, strict all-target Clippy, formatting and diff checks. Sealed
input and historical artifact digests remain unchanged. No push, deployment
or default promotion.

## Evidence and reproduction

Create-only artifacts are under
`outputs/research/waypoint_direct_obstacle_discrimination_20260925/`:
`preservation/`, `development_run_a/`, `development_run_b/`, `freeze_run_a/`,
`freeze_run_b/`, `fresh_run_a/`, and `fresh_run_b/`. Each evaluated case retains
its full generic generation summary under `cases/CASE_ID/generation/`;
the gate summary carries counterfactual clearance/contact evidence and
separate basis/row rejection reasons. Identities exclude output paths and
wall-clock timing.

Run the opt-in `pd-eval` subcommands through `rtk cargo run -p pd-eval --`:

```text
waypoint-direct-obstacle-development --output-dir NEW_DIR
waypoint-direct-obstacle-freeze --development-summary DEV/summary.json --output-dir NEW_DIR
waypoint-direct-obstacle-fresh --development-summary DEV/summary.json --code-freeze-summary FREEZE/summary.json --output-dir NEW_DIR
```

Parents of output directories must already exist. Failed development,
tampered/binding-mismatched freezes, changed source/inputs, or existing output
paths cannot open the fresh gate. Runtime evaluation errors are recorded
per case and do not suppress later fresh outcomes; source-freeze drift stops
physical evaluation and explicitly marks remaining cases not evaluated.

No commit, push, deployment, default promotion, controller changes or
operational waypoint composition were performed in this pass.
