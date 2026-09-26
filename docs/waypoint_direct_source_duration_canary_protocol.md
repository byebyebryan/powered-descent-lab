# Flat direct-leg source-duration canary protocol

This opt-in evaluator canary runs the bounded, predeclared next pass for three
already-frozen flat candidates. It asks whether changing only the source-bridge
duration, on a fixed grid and with the existing source handoff and environment,
produces a launchable wrapper that also survives both execution cadences. It is
not a general candidate search, a planner change, or a V2 recertification.

## Frozen inputs and family

Before any new `SimulationState` is constructed, the evaluator rebuilds and
checks the six sealed source artifacts, all nested input-gate bindings, and the
coupled-thrust audit. The pinned semantic IDs are:

| Input | Identity |
| --- | --- |
| Primitive baseline | `fnv1a64:d3fa6b24336f7c05` |
| Topology sweep | `fnv1a64:1bcd5a3bd6c6da01` |
| Nominal plant | `fnv1a64:9e8cbc902ca11fbc` |
| Nominal input manifest | `fnv1a64:9e32aa0c1a52b28b` |
| Launch feasibility | `fnv1a64:2c7b965ffdc809d6` |
| Launch input gate | `fnv1a64:a4654d06c9166b14` |
| First-contact audit | `fnv1a64:f5a6600e99cd283b` |
| Contact-audit input gate | `fnv1a64:86861aa3e4f8b35e` |
| Flat candidate closure | `fnv1a64:f3fba9290c9073be` |
| Flat-closure input gate | `fnv1a64:d7476fd0e0827260` |
| Coupled-thrust audit | `fnv1a64:59cbe1bd2dd9b8d2` |
| Rebuilt source-duration input gate | `fnv1a64:099d25787a2694f1` |

The only bases, in fixed order, are native
`fnv1a64:dee613017622ca16`, research-shortest
`fnv1a64:4e6c0b23f9eb1b8f`, and third
`fnv1a64:a18a98ad6e334014`. The original
source-bridge counts are 1,740, 1,740, and 1,920 physics ticks, respectively.
Each basis tries only `n + [-240, -180, -120, -60, 0]` ticks: five durations
per basis, 60 ticks (0.5 seconds at 120 Hz) apart, and exactly 15 total rows.
There is no adaptive tuning, candidate substitution, or failure-row omission.

For each row, the evaluator first materializes the exact unlaunched source
bridge from the frozen source-pad-rest state to the original handoff, even if
the bridge is `NotCertified`. It derives that row's launch target from the
bridge's first nonzero thrust vector using the plant convention
`atan2(thrust.x, thrust.y)`. It then applies the same 60 upright and 12
candidate-target full-throttle physics ticks. The candidate-specific target is
the only launch-rule variation. The bridge is re-solved from measured launch
position and velocity to the pinned handoff with the row's tick count.

The source handoff, coast, terminal bridge, policy, vehicle, flat terrain,
contact rule, and original V2 candidate classification stay frozen. Every
reseeded bridge is screened for robust thrust (13.32 m/s²), minimum throttle,
powered slew, endpoint, source clearance, source attitude, launch-end-to-first
powered-source-tick slew, aggregate fuel, and mission time. Only analytical
survivors reach the 120 Hz contact-free source-handoff gate; only its survivors
reach full 120 Hz and held-command 60 Hz flights. A failed stage records a skip
reason and does not advance that row to the next stage.

The original candidate's V2 classification is retained separately. A
duration-specific unlaunched bridge or launch-reseeded wrapper is not called a
V2 `Certified` candidate and cannot change the original classification. The
analytical source-clearance mirror retains the known V2 footprint-sign
ambiguity; actual contacts and first-contact predicates are read from the
authoritative core simulation. Neither contact model is changed.

## Result

The no-physics preflight accepted every pinned source identity and binding,
including coupled audit `fnv1a64:59cbe1bd2dd9b8d2`, and returned exactly the
three candidates and ordered 15-row family. The two final reference summaries
and a fresh replay from the integrated tree are byte-identical:

| Output | Semantic identity | SHA-256 |
| --- | --- | --- |
| `outputs/research/waypoint_direct_source_duration_canary_20260924/run_b/summary.json` | `fnv1a64:dfe0f0feaa15fc24` | `a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c` |
| `outputs/research/waypoint_direct_source_duration_canary_20260924/run_c/summary.json` | `fnv1a64:dfe0f0feaa15fc24` | `a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c` |
| `outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json` | `fnv1a64:dfe0f0feaa15fc24` | `a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c` |

The semantic identity hashes the serialized evidence with its own identity
field cleared; paths and wall-clock values are not fields of the artifact.
`run_a` is an earlier provisional output and is preserved, not used as the
final identity or byte-comparison reference. The final family proof records 15
predeclared and 15 recorded rows, zero omitted rows, and 15 rows with at least
one failed screen or downstream outcome. That failure count deliberately
includes a held-60 strict handoff miss even when that same lane later lands.

| Basis (original `n`) | Duration rows reaching analytical + 120 Hz source handoff | 120 Hz full flight | Held-command 60 Hz full flight |
| --- | --- | --- | --- |
| Native (1,740) | `−180` (1,560), `−120` (1,620) | Both landed on target | Both landed on target; both miss strict source handoff |
| Research-shortest (1,740) | `−240` (1,500), `−180` (1,560), `−120` (1,620), `−60` (1,680), `0` (1,740) | All five crashed at first contact | All five crashed; all miss strict source handoff |
| Third (1,920) | `−180` (1,740), `−120` (1,800) | Both landed on target | Both landed on target; both miss strict source handoff |

The other six rows (`−240`, `−60`, and `0` for native and third) were retained
and screened out by the re-solved bridge's coupled-thrust cap. Their peak
requested accelerations range from 13.3988 to 13.6054 m/s², above the
13.32 m/s² robust cap. Their minimum-throttle, powered-slew, endpoint,
source-clearance, source-attitude, launch-boundary, fuel, and time screens
passed. The nine analytical survivors requested 12.5479–13.2187 m/s², with
minimum nonzero throttle 0.6358–0.7157 and maximum powered slew
0.0267–0.0592 rad/s; all recorded component and aggregate screens passed.

The original V2 candidate classifications remain `Certified` for all three
bases. At original duration (`0` offset), the unlaunched bridge identity and
the frozen launch/reseeded-bridge parity matched for each basis. The separately
reseeded source-bridge primitive classification is not a substitute for the
original: it is `NotCertified` for native and third at that duration and
`Certified` for research-shortest. Across all 15 launch rows, the stored
candidate target matched the unlaunched bridge's first powered direction, and
measured launch position/velocity matched the reseeded bridge start. In
particular, the unlaunched `NotCertified` bridge still supplied the
candidate-specific tilt.

All 18 flown lanes recorded an authoritative first contact and passed the
paired contact-replay trace check. Core and replay classifications agree for
all rows: 10 `crash` contacts and 8 `stable_touchdown_on_target` contacts.
Four of the nine held-60 lanes landed, but all nine held-60 rows miss the
separate strict 1e-6 m / 1e-6 m/s source-handoff diagnostic: position error is
0.1503–0.2789 m and velocity error is 0.0206–0.0383 m/s. The four stable
held-60 landings do not erase those misses. The artifact retains the actual
first-contact states and core predicates, contact events, per-step command and
state evidence, saturation counters, termination state, source-handoff
errors, and 120 Hz source-gate-to-flight prefix checks. It does not infer
contact from the V2 footprint mirror.

## Reproduction and validation

Use the frozen relative input paths shown below and choose two output
directories that do not already contain `summary.json` (the evaluator refuses
to overwrite one):

```text
cargo run -p pd-eval -- waypoint-direct-source-duration-canary \
  --baseline-summary outputs/research/waypoint_direct_contact_contract_20260924/baseline/summary.json \
  --sweep-summary outputs/research/waypoint_direct_contact_contract_20260924/sweep/summary.json \
  --nominal-summary outputs/research/waypoint_direct_contact_contract_20260924/nominal/summary.json \
  --launch-summary outputs/research/waypoint_direct_contact_contract_20260924/launch/summary.json \
  --contact-audit-summary outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json \
  --flat-canary-summary outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json \
  --coupled-audit-summary outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json \
  --output-dir outputs/research/waypoint_direct_source_duration_canary_20260924/run_e
```

The same command with a second fresh output directory is the byte-identity
check. Append `--preflight-only` for the no-physics binding/family gate.
Focused source-duration tests passed (6 tests); `cargo check -p pd-eval
--all-targets` passed. Fresh legacy replays preserved the sealed launch,
flat-closure, and coupled-audit summaries byte-for-byte:

| Frozen summary | Preserved SHA-256 |
| --- | --- |
| Launch feasibility | `deab83d3af4cb96b0c608a1266912fc83c1d96bdba750b7610470e74b0627174` |
| Flat candidate closure | `18898544e49f2b8452e47ab4d206761a03ec54602a2a4b429e6c244e2e90bba1` |
| Coupled-thrust audit | `4a40e971c2b67f6ac395ad8651c99cfff9fa6ae7c9c4611a55a7987a5bff4226` |

Those comparison outputs were written under
`/tmp/pd-source-duration-regression-20260924/{launch,flat,audit}`. No
production planner, simulator/contact behavior, controller, F6 path, default,
or earlier artifact was edited. The integrated tree passed `cargo test
--workspace` (650 tests across 12 suites), `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff
--check`. The fresh `run_d` replay from that tree retained the reference
SHA-256 exactly.

## Next branch

The fixed-launch/fixed-policy flat feasibility question now has positive
witnesses: nine rows pass analytical and 120 Hz source-handoff gates, and four
of those land at both cadences. The next study should investigate a
held-command-aware 60 Hz handoff/selection contract, including the authoritative
core first-contact screen, before any held-out transfer or planner promotion.
Do not loosen the 1e-6 strict diagnostic because a held-60 lane landed. Keep
the V2 footprint-sign ambiguity as an explicit cross-model prerequisite; this
canary does not resolve it or justify changing either model.
