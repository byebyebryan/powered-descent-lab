# Nominal ballistic-direct flight integration results

The opt-in flight integration passes its declared gate: all 24 exposed
body-aware checkpoint cases regenerate exactly and land through the ordinary
controller/simulator runner. Two independent release runs agree. This closes
the complete accepted-program-to-flight seam, not production planner defaults,
perturbation robustness or waypoint search.

The [protocol](nominal_direct_flight_integration_protocol.md) owns this pass.
The preceding [body-aware terminal checkpoint](waypoint_direct_body_aware_terminal_results.md)
and all historical artifacts retain their original evidence and classifications.
The ten previously fresh cases are now exposed regression controls; this pass
does not claim new held-out coverage.

## Capability and unchanged boundaries

- `pd-core::FlightProgramV1` binds the complete timed program to the physical
  context, mission, pads, policy identities and accepted witness. It validates
  clock coverage, commands and phase bounds before execution. Structural
  validity alone is not a physical safety certificate.
- `pd-control::run_flight_program` plays the exact commands through unchanged
  `run_controller` / `run_simulation`, not a second flight engine. Research
  post-step indices 1, 3, 5, ... become ordinary pre-step callbacks 0, 2, 4, ... .
  Commands remain held on the original global 60 Hz clock under 120 Hz physics.
- `pd-eval::run_nominal_direct_flight` generates from mission inputs, selects
  only complete accepted witnesses, independently verifies the selected
  witness, executes the program and independently replays the action log.
  Accepted-only ranking remains planned total ticks, then stable witness identity.
- Typed `Unknown`, `Unsupported` and `Invalid` decisions execute no flight.
  Unknown means finite-family exhaustion, not impossibility or waypoint demand.
  There is no silent V1-controller or waypoint fallback.
- The generator, source fitting, terminal reference/executor policy, safety
  thresholds, physics and existing built-in controllers are unchanged. This
  mode is not registered as a default controller and does not change
  `pd_plan::plan()`.

Output roots are create-only. Direct bundles retain the request, scenario,
policy, decision, generated ledger, complete program, selected witness,
independent contact/safety audit, ordinary manifest/actions/events/samples,
controller telemetry, performance and a generic HTML report. The report has no
misleading V1 chord plan. Failed generation/execution retains inputs and
proposed evidence; non-Direct bundles contain no executed flight.

## Exact-parity acceptance

| Exposed family | Cases | Available / accepted schedules | Result |
| --- | ---: | ---: | --- |
| Uncut flat/uphill/downhill, 600 / 1,000 m | 6 | 76 / 76 | Exact generation and ordinary flight parity |
| Flat/low/high/late-broad, 700 / 900 m | 8 | 76 / 76 | Exact generation and ordinary flight parity |
| Previously sealed flat/uphill/downhill/high/late-broad, 750 / 950 m | 10 | 100 / 100 | Exact generation and ordinary flight parity |
| Total | 24 | 252 / 252 | Passed independently twice |

Each run records all 480 predeclared source rows, 228 source-stage skips and
302 attempted terminal durations. Every regenerated full case artifact matches
the archived case exactly, including accepted counts, selected row, commands,
reference and contact evidence. Ordinary execution has exact command/clock,
first-contact tick and fuel parity, safe target landing and deterministic
independent action replay. No first-step crash or phase-local clock reset occurs.

Across the repeats, 288 file pairs match byte-for-byte: request, scenario,
terminal policy, decision, generation, program, witness, safety audit, manifest,
actions, events and samples for each case. All 24 controller-update logs match
after removing only per-update compute timings. All 24 flight summaries match
after zeroing only the five observational wall-time fields. Performance JSON
and HTML containing timings are not byte-parity claims.

Both regression roots have identity `fnv1a64:3ea6e074f6de460e`. Their 119-file
source/input/protocol binding is unchanged before, between and after all cases:
`695ed2f1a1068bfd0bedb8be1f72a6b8e37f47c6c4d7a6064b76a3ee0ee3f7fe`.
It includes all six crates' Rust source and manifests, root Cargo files, the
integration protocol, three physical manifests, the embedded V2 vehicle fixture
and original flat-scenario input. The old whole-source freeze is historical,
not reused as current integration authority. Results/README/progress prose is
written after measurement and is not part of this runtime source binding.

All 26 archived comparison files retain their pinned SHA-256 digests. The two
archive roots remain:

- Development: `f828673081ab4bd770eca3646279c719aab7f0cc550a3460e607809ef0022799`.
- Previously fresh: `60fe4e60737b780dbba61bce583e401df732e9b355b2a93c1891b4ddeeab805d`.

## Measured release cost

The table covers 48 observations: two sequential runs over the same 24 inputs.
Compilation and the full workspace suite had finished before measurement.
These are local observational wall times, not a performance contract or an
optimized planner benchmark.

| Stage, whole case/flight | Minimum | Median | Maximum |
| --- | ---: | ---: | ---: |
| Input-driven research generation | 338.133 ms | 943.888 ms | 1,367.203 ms |
| Independent selected-witness verification | 239.858 ms | 661.560 ms | 956.726 ms |
| Ordinary controller/simulator execution | 1.432 ms | 1.862 ms | 3.963 ms |
| Independent ordinary action replay | 0.518 ms | 0.663 ms | 1.494 ms |
| Artifact and report writing | 7.020 ms | 9.837 ms | 13.172 ms |

The complete 24-case gates take 45.746 s and 45.767 s, including archive parsing,
hashing, source checks and case comparison. The execution/replay rows time the
entire offline simulated flight, not an individual controller callback. Writing
excludes the final performance/summary file writes and regression-root overhead.

Generation still performs original-policy comparison flights, source fitting
and full terminal-family validation. Selected verification additionally
regenerates the original source family. Setup cost therefore dominates ordinary
playback; this pass measures that research backend without optimizing it.

Per 24-case run, known terminal-attempt ordinary/neutral verification advances
2,342,688 physics ticks; selected complete-witness ordinary/neutral replay
advances 223,068; the new ordinary executor and independent action replay each
advance 111,534. The first two are explicitly **subtotals**: source-fitting,
comparison and other generation work are not fully instrumented. Total
generation physics ticks remain `null`, not a mislabeled bound or subtotal.
Timings never affect decisions, selected programs or deterministic identities.

Measurement uses release `pd-eval` on local x86_64 Linux, Rust 1.98.1
(`48a229cea`, Arch Linux package `1:1.98.1-1`). Binary SHA-256 is
`e1c04223ca7ab2ed7028576ea5c2bea77f149801c6a48cdafc9599e424541be6`.

## Validation and CLI smoke

- Fifteen focused adapter/gate/CLI tests pass. Neutral core/control tests also
  cover serialization, clock coverage/hold, binding/payload rejection,
  unexpected end and repeatability.
- Negative coverage includes invalid/ambiguous pads, unsupported policy,
  physics, vehicle or source state, finite horizon/terrain exhaustion and a
  finite self-rehashed command alteration rejected before motion. Finite
  Unknown does not add a waypoint or relabel itself Unsupported.
- The full workspace suite passes **766 tests in twelve suites**, with one
  explicitly ignored historical-artifact integration test. It runs in a
  source-matching staged checkout with Git metadata, a separate target directory
  and no retained `outputs/` present at startup. Historical-output-dependent
  acceptance is the separately invoked 24-case regression command above.
- Strict all-target workspace Clippy, formatting and diff checks pass.
- The ordinary mission-input CLI accepts the flat-600 scenario with its exact
  IDs `pad_source` / `pad_main`, reproduces the regression command payload and
  lands safely at tick 3342 / 27.85 s. Its identity is
  `fnv1a64:9200afe5a181abaa`; read-only preflight creates no simulation.
  Reusing its output root is refused before generation or overwrite.
- An initial smoke invocation with mistaken `source` / `main` IDs is retained
  as an Invalid bundle: zero generated rows, no program, simulation or manifest.
  Correcting the IDs uses a new output root; failure evidence is not overwritten.

Create-only artifacts live under
`outputs/research/nominal_direct_flight_integration_20260928/`:
`gate_a/`, `gate_b/`, `cli_flat600_direct/` and the retained Invalid
`cli_flat600/`. Each Direct case has `report.html` and the full JSON bundle.

```bash
cargo run --release -p pd-eval -- nominal-direct-flight-regression --preflight-only
cargo run --release -p pd-eval -- nominal-direct-flight-regression --output-dir NEW_GATE_ROOT
cargo run --release -p pd-eval -- nominal-direct-flight \
  --scenario SCENARIO.json --source-pad-id SOURCE_PAD_ID \
  --target-pad-id TARGET_PAD_ID --output-dir NEW_FLIGHT_ROOT
```

Replace the uppercase arguments with a supported complete `ScenarioSpec`, its
exact pad IDs and a new output directory. Use `--preflight-only` instead of
`--output-dir` for read-only input validation. The regression command requires
the retained immutable archive; ordinary generation/flight does not.

## Verdict and remaining boundary

The accepted nominal direct transfer is now an executable opt-in capability,
not only an evaluator result or route label. Close this integration pass here.
Production V1/default behavior and the roadmap remain unchanged.

Follow-up review finds no blocking defect within this nominal scope and makes
no runtime-source change. All 24 focused contract/playback/adapter/gate/CLI tests
pass again, along with strict all-target workspace Clippy, formatting and diff
checks. The 119 bound files still match the artifact-free 766-test validation
checkout. Both retained 24-case runs, 288 deterministic file pairs and all
timing-normalized flight summaries/controller logs are rechecked; current source,
release binary and historical archive digests remain unchanged. A redundant
full-suite or flight rerun is not needed for these documentation-only review notes.

The reviewed source/documentation checkpoint is committed. Generated bundles
remain local ignored artifacts, not files embedded in the commit. No push,
deployment or default promotion is part of this closure.

Selected hull-penetration margins are still only 6.29–7.50 mm, unchanged from
the body-aware checkpoint. Pointwise discrete nominal success is not swept-path
proof, perturbation robustness or arbitrary mission coverage. Robust contact
and a useful setup-cost budget remain gates before default/production promotion.
Waypoint search/composition and arbitrary incoming waypoint states are separate
unimplemented capabilities. No obstacle here proves a waypoint is necessary;
finite-family exhaustion remains Unknown. Further source refitting, cadence-only
experiments or threshold relaxation are not required to close this pass.
