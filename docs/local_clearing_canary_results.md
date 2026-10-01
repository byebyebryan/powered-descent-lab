# One obstruction local clearing results

## Verdict

The sealed evaluator-only experiment passes all five gates in two corrected
final-source runs. Its verdict is `LocalClearingThenTerrainBlocked`: a local
maneuver produces a real, safely replayed airborne handoff above the original
obstruction, and the unchanged nominal generator produces a fresh continuation
from that state. The independent terrain audit blocks that continuation again
on the same broad plateau. No blocked suffix is executed on the active flight.

This establishes the first local clearing piece and its replanning seam, not
landing, clearance of the whole plateau, a second waypoint, or a usable complete
planner. The [protocol](local_clearing_canary_protocol.md) and
[selection manifest](../fixtures/research/local_clearing_canary_inputs_v1.json)
preceded measurements. The maneuver family, ranking, 5 m reserve, vehicle,
terrain and original deadline were not tuned after observing results.

## Baseline and intervention

The unchanged 900 m flat control lands safely without a floor cutaway. The flat
and late/broad terrain cases have identical complete canonical searches and
selected commands. Their fixed nominal whole-flight peak is 232.373 m.
The obstructed case first violates body reserve at tick 2354, or 19.6167 s,
at (-350.864, 221.131) m, then contacts terrain at tick 2358. This is the expected
interior obstruction, not a source launch or first-step crash.

All four actual pre-conflict entries are admitted. Each is collected through an
ordinary canonical prefix before consuming its pending command; an independent
guarded whole-source run has matching complete state, contact, actions, events
and samples. The entry snapshots are evidence, not executable state restores.

| Entry | Global tick | Time in seconds | Physically propagated rows | Locally accepted rows |
| --- | ---: | ---: | ---: | ---: |
| First idle hold | 1994 | 16.6167 | 42 | 0 |
| Source bridge 75 percent | 1512 | 12.6 | 42 | 2 |
| Source bridge 50 percent | 1032 | 8.6 | 42 | 0 |
| Source bridge 25 percent | 552 | 4.6 | 42 | 0 |

The search retains all 168 rows and all 60,480 handoff-boundary decisions. There
are 217 eligible boundaries, 41,375 insufficient-progress decisions, 18,837
incomplete-prefix decisions and 51 unsafe-continuation decisions. These are
correlated windows of the same finite traces, not independent missions.

All 42 late idle-entry traces stop at a reserve violation. Across the earlier
entries, 102 traces stop at reserve violations, six leave the terrain domain and
18 reach the finite trace bound. The two accepted rows belong to the 75-percent
entry and use upright full-cap boosts lasting six and eight seconds. Finite
rejection of other rows is not a physical impossibility proof.

## Selected clearing maneuver

The declared ranking selects `source_75_percent_row_26`: latest eligible entry,
earliest handoff, least actual fuel burn, then stable row identity. Both accepted
rows first qualify at tick 2820; the six-second boost wins the fuel tie-break.
Later nominal generation, terrain audit and landing never enter this selection.

The selected maneuver starts at tick 1512, commands upright thrust acceleration
13.32 m/s² for 720 physics ticks, then explicitly commands idle/upright at every
global 60 Hz update. Its actual mass and held-pair throttle conversion determine
fuel use; acceleration is not substituted for physical actuation. It coasts
588 ticks before the selected handoff. Rotation from the actual powered entry
and every fuel update are propagated by the unchanged core.

| Milestone | Global tick | Time in seconds | Position x and y in meters | Velocity x and y in meters per second |
| --- | ---: | ---: | --- | --- |
| Actual powered entry | 1512 | 12.6 | -755.228, 158.748 | 36.982, 16.361 |
| End of local boost | 2232 | 18.6 | -525.506, 318.675 | 38.313, 37.166 |
| Actual handoff | 2820 | 23.5 | -337.771, 382.821 | 38.313, -10.903 |
| Separate idle certificate endpoint | 3060 | 25.5 | -261.145, 341.313 | 38.313, -30.523 |

The handoff has upright attitude, zero angular rate, idle held command and
5,682.280 kg fuel. Actual local fuel burn to handoff is 214.102 kg. Its x position
passes the declared progress threshold of -338.057 m, derived from the original
conflict plus the 12.806 m conservative body diameter. This is useful progress
out of the original low trajectory, not passage beyond the feature's far edge.

The local trajectory through the separate two-second certificate stays above
the full 5 m body reserve. Its minimum measured body clearance is 21.313 m.
The unchanged airborne admission conditions hold at every aligned certificate
boundary. Both whole-source bounded endpoints stop `CoverageExhausted`, with
the mission still Flying/InProgress/Running and no incoming contact. Complete
final snapshots, contact options, actions, events and samples agree across
ordinary live execution, independent bounded source runs and official replay.
The original hard deadline remains tick 9600, or 80 s.

The active branch remains at H, tick 2820. Its separate guard clone is not
consumed before regeneration. The selected search trace later violates reserve
at tick 3120; that later failure is outside the completed certificate and does
not invalidate it. The certificate does not establish eventual landing,
indefinite viability, disturbance tolerance or safe motion during computation.

## Fresh replanning and the next obstruction

Regeneration receives the retained actual H and original context/deadline, with
no saved suffix, restored snapshot or clearing-row seed. The unchanged airborne
family evaluates 56 rows: 17 produce nominal proposals and 39 are rejected.
Its terrain-blind ranking chooses 530 coast ticks followed by a nominal
1,056-tick terminal window, with planned contact at tick 4406. The nominal
proposal's peak is its incoming COM height, 382.821 m.

Actual terrain rejects this fixed choice during coast. The first reserve
violation occurs at tick 3120, or 26 s, with 4.803 m clearance versus the required
5 m. The diagnostic branch contacts the 315 m plateau at tick 3136, or 26.1333 s,
at (-236.880, 319.989) m, before reaching its nominal terminal phase.

Ordinary/neutral audit parity and an independent combined whole-source prefix
proof agree on the complete final and incoming-contact states, consumed commands,
actions, events and samples. This is genuine `TerrainBlocked`, not an unsupported
handoff or exhausted nominal family. The unsafe fixed suffix is audited and
replayed only on private diagnostic branches; the active flight does not crash
there. No higher nominal proposal is selected after terrain rejection, and no
Direct landing proof is claimed.

This obstruction pair exercises the blocked regeneration branch. The optional
Direct landing branch has no new physical acceptance witness in these runs.

This result supports the intended piecewise separation: a clearing waypoint can
be useful without guaranteeing an immediately clear direct landing. It also
exposes the next required behavior: another correction may be needed while
still above the same terrain feature. Two seconds of safe continuation alone
does not make the rest of the mission safe.

## Evidence correction and repeatability

The earlier roots named `final_a` and `final_b` are preserved but are preliminary,
unaccepted local-clearing evidence. Both stopped at `BaselineEvidenceFailure`
before collecting local entries or running local search. Their flat control,
canonical search equality, actual obstruction and independent preservation all
passed. Their complete repeat comparison passes in `initial_repeat.json`.

The defect was an evaluator predicate, not flight physics. The legacy audit's
`commands_match` flag means all nominal commands were consumed and the complete
planned endpoint matched. It is correctly false when terrain causes early
contact. The new baseline incorrectly demanded that full endpoint for a blocked
flight. The correction retains the old audit semantics and instead requires
independent agreement on the command prefix actually consumed, complete state
and contact, and ordinary/official replay. The same distinction now applies to
blocked regenerated suffixes. A structural negative accepts a matching truncated
prefix and rejects altered commands or full contact evidence.

Accepted create-only roots are:

- `outputs/research/local_clearing_canary_20260930/final_prefix_v1_a`
- `outputs/research/local_clearing_canary_20260930/final_prefix_v1_b`

Both have envelope identity `fnv1a64:35d730ecc296b846` and complete experiment
identity `fnv1a64:9767b71c0a398902`. Their 143-file recursive source/input/protocol
closure is unchanged, with identity
`9ee8f15ec48bf481d7dad4f903b0a3a60cce954a3178091ad02c3c38cf7370b6`.
The protocol seal remains
`1948d70544280c60214503d4fd7fb2ce1c4159ce88e7617eaefce0711c4e82e8`;
selection seal remains
`178c38890c91d3a6bb611697418668a1639dae9093626d50a95a5a173f9c3702`.
Only the evidence check and recording changed before the first local search.

The final complete repeat comparison passes and is retained in `final_repeat.json`.
It compares complete experiment/preflight/summary payloads and rereads every
preservation output, including static HTML shells and embedded physical data.
Only the explicitly sealed observational timings and output-root literals may
differ; full source envelopes and deterministic identities remain compared.
The new physical experiment has no timing exclusions.

Both runs independently preserve all eight canonical controls, all four complete
prior airborne case payloads containing twelve continuations, and all 24 prior
source-rest controls. The historical comparison retains 8 canonical case
comparisons, 4 airborne case comparisons and all 410 source-rest output files;
the repeat compares all 457 files across the preservation trees. Legitimately
expanded historical source envelopes and their derived outer identities are
the only non-timing historical exceptions. No proposal, full state/contact,
command, physical decision or replay payload is exempted. The legacy runner
also retains its 26 declared historical archive files unchanged.

The corrected final runs use an optimized build; the preliminary roots used a
debug build. Timing fields are observational, and these runs are not a planner
latency benchmark. Planning pauses the offline simulator.

## Validation and next boundary

The final integrated tree passes 861 workspace tests across twelve suites, with
one existing explicit historical-artifact ignore. Nineteen added focused tests
cover the sealed policy, local proof, prefix-versus-endpoint semantics and
evidence harness. Strict all-target workspace Clippy, formatting and whitespace
checks pass. CLI checks require an explicit new output directory and expose no
policy-tuning or promotion switches. Structural tests do not extend physical
obstacle coverage beyond this previously exposed input pair.

Close this one-clearing-maneuver foundation. The next bounded design question is
one additional clearing/replanning iteration from the retained H, where the new
fixed nominal route is known to be blocked. Begin at the still-safe H, not the
later conflict/contact snapshot; preserve the original clock, fuel, deadline,
body reserve and local-only selection. Explicitly cap the number of corrections
and distinguish repeated conflict on the same feature from useful progress.
That continuation pass is recommended, not implemented or automatically
authorized here. Immediate landing must not become a local candidate gate.

Further bounded work is justified by demonstrated local execution and supported
regeneration, rather than a new source-launch or landing-physics hypothesis.
Production integration, repeated waypoint execution, arbitrary handoff states,
real-time computation, robust viability and broader terrain coverage remain
open. No default, core/contact physics, controller, existing generator policy or
completion reserve changed. The implementation pass left the work uncommitted
for separate review; no push or deployment occurred. The subsequent review and
commit checkpoint is recorded in [progress](progress.md).

Primary owns the policy, physical engine, acceptance, prefix-proof correction,
integration review and measurements. Luna supplies the bounded canary/CLI and
preservation harness, corrections, tests and the read-only semantic audit.
