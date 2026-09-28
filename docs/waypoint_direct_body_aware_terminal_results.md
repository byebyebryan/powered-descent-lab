# Body-aware nominal direct terminal results

The bounded prototype passes development and the newly sealed capability gate:
all 14 exposed cases and all ten fresh cases have complete accepted Direct
flights without a floor cutaway or waypoint. This closes the nominal terminal
reference/execution question, not production planner integration or robustness.
The [protocol](waypoint_direct_body_aware_terminal_protocol.md) defines the
fixed policy, pre-implementation seal, staged gates and evidence boundaries.

## What changed, and what did not

The new evaluator policy keeps the input-generated launch/source commands and
source handoff unchanged. It replaces the terminal reference with quadratic
horizontal / affine vertical net acceleration, adding a zero final horizontal
acceleration constraint so thrust and body attitude approach upright at contact.
The endpoint deliberately crosses the pad by 5 mm at 1.5 m/s downward speed;
acceptance audits actual first core contact, not just that endpoint.

Adjacent reference thrust vectors are averaged and their scalar throttle is
inverted against both post-burn masses. Commands are held on the original
global 60 Hz clock under 120 Hz physics. The coast rotates the real body toward
the tail's entry attitude and may add one real idle tick for clock alignment.
There is no attitude teleport, clock reset, feedback retuning, source refit or
per-case policy adjustment. Seven duration offsets range from zero to +3 s.

Complete stored programs are independently replayed from source-pad rest in
ordinary and neutral core states. Both must agree on authoritative first
contact, including the ordinary stable-touchdown zeroing exception. Neutral
evidence preserves incoming velocity and angular rate. Actual body clearance,
terrain domain, source handoff, fuel, slew, thrust and mission budgets all pass.
No core contact/clearance threshold, controller frequency, production
`pd_plan::plan()` behavior, V2/F6 classification or default has changed.

## Development: unchanged historical inputs and source programs

| Exposed family | Cases Direct | Source schedules | Original accepted | New accepted |
| --- | ---: | ---: | ---: | ---: |
| Uncut flat/uphill/downhill, 600 / 1,000 m | 6 / 6 | 76 | 28 | 76 |
| Flat/low/high/late-broad obstacles, 700 / 900 m | 8 / 8 | 76 | 34 | 76 |
| Total | 14 / 14 | 152 | 62 | 152 |

All 62 previously accepted rows remain accepted. Every one of the 152 available
source schedules obtains a new tail; 128 source-stage skips remain recorded in
the full 280-row family. There are 190 attempted terminal durations, with all
later offsets explicitly unattempted after each row's first acceptance.

The two previously Unknown high cases now have four and two accepted witnesses.
Both selected witnesses are row 16, use +180 terminal ticks (+1.5 s) and one
additional coast alignment tick. Their planned/contact times are
`66.583333 / 66.55 s` and `70.283333 / 70.25 s`. Incoming normal speed is about
`1.5 m/s`; contact tilt is about `1e-5 rad`, instead of the old approximately
`0.06 rad` ideal contact tilt. Their upper feet are slightly below the pad,
not approximately half a metre above it.

All fourteen regenerated original full-family summaries reproduce their pinned
historical bytes. The old eight-case ledger remains exactly 34 accepted / 42
rejected; the new 76 / 0 terminal result is separately identified. The
[September 27 diagnostic](waypoint_direct_terminal_admissibility_results.md)
retains its original results and counterfactual 120 Hz evidence. Historical
failures are not overwritten or relabeled as new-policy successes.

## Sealed cases: no post-freeze retuning

The complete manifest was sealed before implementation. After primary
development acceptance and source freeze, all ten cases were evaluated in the
declared order: 750 m, then 950 m; flat, uphill +100 m, downhill -100 m, high
obstacle and late/broad obstacle. Terrain is continuous and uncut. These are
held-out parameters within the studied family, not arbitrary terrain coverage.

| Profile | 750 m accepted / scheduled | 950 m accepted / scheduled | Selected contact time, 750 / 950 m |
| --- | ---: | ---: | ---: |
| Flat | 13 / 13 | 11 / 11 | 29.85 / 33.10 s |
| Uphill | 14 / 14 | 14 / 14 | 28.85 / 32.10 s |
| Downhill | 15 / 15 | 15 / 15 | 29.85 / 34.35 s |
| High obstacle | 2 / 2 | 2 / 2 | 66.366667 / 72.283333 s |
| Late/broad obstacle | 8 / 8 | 6 / 6 | 36.10 / 38.85 s |

The 200-row fresh family retains 100 source-stage skips and 100 available
schedules. All 100 schedules are accepted under the new policy, versus 54
under the original policy generated as a separate comparison. There are 112
attempted terminal durations. Both high cases again use +180 ticks and one
real coast alignment tick. No source-first-step crash, helper error or
artifact-verification failure occurred.

## Margins and limits

Across the 24 selected development/fresh witnesses, upper-foot-clearance margin
is about `0.154 m`, and normal-speed margin about `1.5 m/s`. Fresh selected
terminal airborne tracking errors are at most `0.179 mm` in position and
`0.0000348 m/s` in velocity under actual held-60 Hz execution.

This does not make every safety margin large. Selected hull-penetration margin
is only `6.29–7.50 mm` under the current discrete core rule. Reference minimum
outside-pad clearance for the exposed high cases is `5.104735 / 5.115870 m`,
and for fresh high cases `5.119740 / 5.204602 m`: the former barely exceed the
5.1 m construction screen. Actual full-program replay retains the unchanged
5 m outside-corridor requirement and nonnegative clearance in the supported
source/descending target-pad corridors. No swept-path or perturbation-robust
certificate is claimed.

A direct flight can safely overfly an obstacle with a different arc and
recoverable landing tail. An obstructed selected arc is not waypoint necessity;
exhausting this finite direct family still means Unknown, not physical
impossibility. Arbitrary incoming waypoint states, waypoint composition,
controller integration, general mission coverage and real-time planning remain
unproven. The evaluator also regenerates full original families and replays
many schedules; its research execution cost is not a usable-planner cost gate.

## Integrity, artifacts and validation

Artifacts are create-only under
`outputs/research/waypoint_direct_body_aware_terminal_20260928/`:

- `development_final_a/` and `development_final_b/`: fourteen cases each;
- `code_freeze/`: primary-accepted development/source/protocol/input binding;
- `fresh_run_a/` and `fresh_run_b/`: ten sealed cases each.

Development's root and all fourteen case summaries are byte-identical across
the two final runs. Fresh's root and all ten case summaries are also
byte-identical across its independent runs. All 114 bound source/input files,
all fourteen historical baseline digests and the preserved diagnostic root
digest match after evaluation. The preliminary
`development_a/` is preserved under its earlier verifier/source binding; all
fourteen candidate case files match the final run exactly, but that root is
not the accepted freeze input.

Reload verification reconstructs the entire finite first-success ledger before
independently replaying accepted commands. This prevents a self-rehashed saved
case from hiding an earlier accepted duration, omitting an accepted row or
falsifying the ranked selection. The independent review found this integrity
gap before freeze; it was corrected without a numerical policy change, and
omitted-witness/forged-duration regressions pass. The reviewer confirmed closure.

Frozen measurement source/input binding: `fnv1a64:76874f8adaeeeb3a`, 114 files.
Protocol SHA-256:
`0dd12e3f8eff5e3be46a8dae45901f99bf9a48c73e647836be6537cf4b5e6669`.
Sealed manifest SHA-256:
`f7b982724cfc8e4df7fd312519a11cfcb1cea25348c7e9451269c8a3e175ddb7`.

Development identity: `fnv1a64:df774901072fdf2e`; summary SHA-256:
`f828673081ab4bd770eca3646279c719aab7f0cc550a3460e607809ef0022799`.
Freeze identity: `fnv1a64:39803cbaab704dcf`; summary SHA-256:
`ac0c96c3c4604c4c947cdf200b9eb38fd8e5f5d51ab326507c892f17c2f5b4ce`.
Fresh identity: `fnv1a64:07a2054fa48ab19a`; summary SHA-256:
`60fe4e60737b780dbba61bce583e401df732e9b355b2a93c1891b4ddeeab805d`.

The measurement tree passed 742 local workspace tests in twelve suites, strict
all-target workspace Clippy, formatting and diff checks. Source/protocol/input
identity is checked before and after every measured case. Stale development
source bindings refuse freeze, and existing output roots/files are not replaced.

Review resolution: the default manifest/source-binding test now uses only
checked-in inputs. The full historical-preflight test is an explicitly ignored
retained-artifact integration gate, and passes when run with those fourteen
local summaries. It is not silently skipped based on whether files happen to
exist. Run it explicitly through:

```text
rtk cargo test -p pd-eval historical_preflight_binds_fourteen_inputs_and_sealed_ten_without_writes -- --ignored
```

This cleanup changes only the runner's `#[cfg(test)]` section. Its non-test
source, the flight engine, protocol, sealed inputs and all measured output bytes
are unchanged. The raw all-source measurement binding above is now historical
relative to the reviewed test source; old development/freeze summaries are not
relabeled or reused under the new binding. No inspected case was retuned or
rerun as a newly held-out experiment.
Review-only all-source binding: `fnv1a64:aac899458335ec47`; this is not a new
physical measurement freeze.

Review validation passed the full artifact-free staged source checkout with
Git HEAD metadata: 742 tests in twelve suites, zero failures and one explicitly
ignored retained-artifact integration test. That integration test passed
separately against the preserved local outputs. Strict all-target workspace
Clippy, formatting and staged diff checks also pass. The review and commit
cover both the preserved causal diagnostic and the new terminal prototype;
there is no push, deployment, planner/controller change or default promotion.

Run through `rtk cargo run -p pd-eval --`:

```text
waypoint-direct-body-aware-terminal-development --preflight-only
waypoint-direct-body-aware-terminal-development --output-dir NEW_DEVELOPMENT
waypoint-direct-body-aware-terminal-freeze --development-summary NEW_DEVELOPMENT/summary.json --output-dir NEW_FREEZE
waypoint-direct-body-aware-terminal-fresh-gate --development-summary NEW_DEVELOPMENT/summary.json --code-freeze-summary NEW_FREEZE/summary.json --output-dir NEW_FRESH
```

The primary owned reference/executor design, physical verification, seal,
evaluation, integration and acceptance. Luna owned runner/preflight/CLI and
their focused tests, plus the bounded read-only acceptance audit. The measured
implementation pass made no commit, push, deployment or default promotion;
the subsequent review checkpoints the source with only test-portability cleanup.

## Verdict and next boundary

Close the nominal terminal rabbit hole here: the added reference freedom and
intended-60 Hz executor resolve the exposed failures and pass all ten sealed
cases. More source fitting, cadence-only experiments or obstacle sweeps are
not required to answer this question.

Next, separately plan a bounded opt-in direct-first adapter using a complete
accepted flight witness. Preserve the
semantics: try the ballistic-direct family first; only escalate to waypoint
construction after the declared direct search returns Unknown, without calling
that proof of necessity. Robustness and measured setup cost are gates before
production/default authority; arbitrary waypoint-state support and composition
are separate capabilities. Do not implement that next step automatically.
