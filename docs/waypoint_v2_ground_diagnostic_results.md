# Waypoint V2 ground diagnostic results

## Verdict

The approved [bounded diagnostic](waypoint_v2_ground_diagnostic_plan.md) is
complete on 2026-10-02. There is a concrete terminal-time construction mismatch,
not another first-step crash or a need for a floor cutaway. The existing uncut
ground constructor still reproduces 8/8 landings. The research family's accepted
coverage remains 2/8; no runtime policy, margin or seed changed.

Recommend a bounded, state-derived terminal-time construction that accounts for
lateral distance and velocity as well as vertical recovery. Keep the existing
reference builder, physical backend and 0.925 thrust budget initially. This is
a recommendation for the next separately approved pass, not a fix delivered
here. The [separate primary review](waypoint_v2_ground_diagnostic_review.md)
accepts the diagnostic and retains the runtime replacement no-go.

## Eight matched-entry comparisons

Each original full program starts from fresh H0 on its actual V2 scenario and
9600-tick deadline. Ordinary execution, neutral consumed-prefix contact and an
independent second replay agree with retained commands, final state and raw
incoming contact. Actual source S also agrees exactly with the canonical source
boundary. Actual terminal T is captured before its first new command; neither
boundary is restored into an executor.

The two references below hold that actual T fixed. The original duration comes
from `canonical.search.selected.terminal_tick_count`, not total flight length.
The alternative uses the unchanged research braking formula, rounded to even
physics ticks. Only the original command schedule is flown in this comparison.

| Case | Original duration s | Research duration s | Original first rejection | Research first rejection |
| --- | ---: | ---: | --- | --- |
| Clear 685 | 12.500 | 18.383 | None | Lateral reversal |
| Clear 735 | 13.000 | 19.350 | None | Lateral reversal |
| Clear 845 | 15.500 | 30.617 | None | Lateral reversal |
| Clear 915 | 17.000 | 41.783 | None | Lateral reversal |
| Uphill 845 | 14.000 | 27.033 | None | Lateral reversal |
| Uphill 915 | 13.500 | 20.633 | None | Lateral reversal |
| Downhill 845 | 15.500 | 24.700 | None | Lateral reversal |
| Downhill 915 | 15.000 | 20.700 | None | Lateral reversal |

All eight rebuilt original references reconstruct the original paired throttle
commands with zero maximum difference. There is no remaining attitude turn at
their actual T. Original terminal peak applied throttle is 0.678302 to 0.688143,
comfortably below 0.925; the full-thrust ground preparation is a separate phase.
Original combined bounds are 14.913 to 15.896 m/s², exact reference maxima are
12.940 to 13.063 m/s², and incoming-mass budget limits are 17.046 to 17.138 m/s².
The conservative bound therefore does not exclude these working references.

All eight alternative references satisfy entry geometry and the finish clock,
and their combined bounds also fit the unchanged thrust screen. Their first
failure is forward horizontal velocity reversal, not excessive thrust. Fuel
estimates and alignment demands are recorded separately; they are not physical
measurements of an alternative flight or permission to skip earlier rejection.

The formula approximates a particular vertical braking profile:
`2 * height / (downward_speed + touchdown_speed)`, with the existing discrete
correction. At a high, slowly descending entry it chooses a long terminal time.
The coupled horizontal reference must then finish at the target with zero
lateral velocity over that same time, and fails the no-reversal predicate here.
The working durations show that other vertical profiles are compatible with
these entries. This is not a proof that the formula explains every rejection
at the research family's different entries. In particular, its two downhill
witnesses remain valid at their own entries.

## Three preselected shadow probes

The matched comparisons demonstrate bound conservatism, while sealed research
ground entries still reject on that bound. The conditional shadow question was
therefore exercised without changing nominal admission. Flat/uphill selection
uses the smallest sealed bound/recorded-limit ratio, then seed ID and entry
index. Downhill uses its existing first-ranked witness. No terrain outcome is
used for selection and there is no retry with a different seed or duration.

| Case | Selected seed / entry | Actual terminal peak throttle | Target-plane and terrain landing | Body reserve | 0.925 thrust budget |
| --- | --- | ---: | --- | --- | --- |
| Flat 845 | `upward_shaping_n2364_b65` / 2 | 0.992645 | Pass | Pass | Fail |
| Uphill 845 | `upward_shaping_n2364_b65` / 2 | 0.992673 | Pass | Pass | Fail |
| Downhill 845 | `upward_shaping_n1970_b45` / 2 | 0.892717 | Pass | Pass | Pass |

The flat/uphill recorded bound ratios are 1.073519 and 1.073630. Only their
conservative terminal bound is bypassed; their original `admissible: false`
is retained. The positive control bypasses no gate. Geometry, forward motion,
alignment, fuel, actual actuator/held-pair realization, absolute deadline,
raw safe contact and independent replay remain active.

Each probe executes unchanged research preparation from fresh H0, independently
reproduces that preparation, then realizes acquisition/coast/terminal commands
from the live state. No predicted entry snapshot is executed. Actual entry
position differences are about six micrometres; command/endpoint replay passes.
The target-plane contact ticks are 3448, 3610 and 3512 respectively. Actual
terminal peak accelerations are 18.282708, 18.301938 and 16.328588 m/s², using
the command propagation's changing mass, not an old flight's mass history.

Body reserve here means the existing phase-specific policy, including its
source and terminal contact-corridor exceptions; it is not a strict five-metre
clearance requirement through touchdown. All three have no measured noncontact
reserve violation or query error. Flat/uphill nevertheless have only about
0.7% raw thrust headroom. Their landings do not establish false rejection under
the unchanged 7.5% budget. A tighter combined bound alone cannot admit those
same near-saturated flights while retaining that budget.

## Remaining acquisition gap and next decision

The six research misses retain comparisons against their existing acquisition
ledgers: old phase durations/live S, research preparation at tick 204, estimated
turn/burn, position/velocity, fuel, upward impulse and recorded reasons. There
are zero new acquisition seeds, slot sweeps or S-continuation witnesses.

The old working source acquisitions end at tick 1812 or 1992; research upright
preparation ends at 204. A state difference at 204 does not prove that extending
upright launch is the fix. Nor do successful old entries prove the research
constant-vector acquisition can establish equally compatible entries.

The smallest supported next experiment changes terminal-time construction only,
first checking the eight proven entries, then the same research ground starts
and retained airborne corpus. Choose time from coupled current-state-to-target
constraints rather than a vertical-only equality; allow adequate vertical
profiles without forcing already-safe motion onto one ideal curve. Freeze a
small terrain-blind candidate budget before execution, not another open-ended
duration grid. If this still leaves ground misses, isolate a mode-specific
acquisition adapter in a later pass instead of changing both mechanisms at once.

Later constructor acceptance still needs 8/8 clear actual-terrain/replay/reserve
success and preservation of all 39 airborne regressions. A usable planner still
needs the unchanged full 32-case repeated-handoff gate, including at least 13/16
ordinary landings and the family/reference/integrity requirements. None of
those denominators is improved by this diagnostic.

## Reproduction and evidence

The thin adapter is a child of the existing characterization evaluator and a
subcommand of `pd-eval`; it adds no standalone reporting framework or runtime
constructor. Run from the repository root with a new output directory:

```sh
cargo run -p pd-eval -- waypoint-v2-ground-diagnostic \
  --corpus outputs/research/waypoint_v2_nominal_characterization_20261001/corpus.json \
  --output-dir outputs/research/waypoint_v2_ground_diagnostic_20261002/NEW_ROOT
```

Accepted final-source artifacts are
`outputs/research/waypoint_v2_ground_diagnostic_20261002/final_verified_a` and
`final_verified_b`. Each has eight controls, sixteen references and three
physical probes. Repeats are reproducibility, not six independent probe cases.
The complete JSON objects and sorted bindings are byte-identical:

- Summary SHA-256: `bc05aca22a6491518c463188b91440198302997e4239d5099d2e855190c535f4`.
- Bindings SHA-256: `56026eeadbba80252d4867b534e7c971c6f58eeabc4a5dd4eaf60b479716218b`.
- Artifact identity: `fnv1a64:48da74d61dfa208c`; all 247 source/input bindings
  verify before, after and independently from disk.

`dev_a` retains a development JSON serialization error. `final_a`/`final_b`
retain matching physical evidence but a stale false preparation-replay flag;
the verified pair corrects that metadata. These earlier roots are not accepted
final evidence and were not overwritten.

Final-source workspace tests pass 889 tests with three ignored. Focused
diagnostic/CLI tests pass all six checks when the two archive-dependent tests
are explicitly included. Normal workspace tests leave those two research
tests ignored, alongside one existing ignore. Corpus/tamper, fake-suite and
frozen 8/16/8 structural/input checks pass; structural checks do not measure
new full-loop flight acceptance. Formatting and whitespace pass.

Unmodified strict Clippy reports the pre-existing unchanged
`route_capability.rs:274` `single_element_loop` lint with rustc 1.99.0. All-target
workspace Clippy passes with only that lint allowed and all other warnings
denied. No unrelated source is edited to silence it.

All 68 current fixture files and 1504 monitored retained artifact files preserve
their pre-pass hashes. The 204 protected source/script/input files and all 51
characterization plus 25 terminal helper bodies remain unchanged. Consequently
the complete unchanged characterization is preserved, not rerun or claimed as
new flight measurement. No policy, terrain, controller, contact, local ranking,
default, commit, push or deployment changes occur.

The primary owns adapter implementation, interpretation and final review.
Luna supplies bounded investigation/assertion guidance and the new focused tests.
