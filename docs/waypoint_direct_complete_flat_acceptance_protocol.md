# Complete flat direct-witness acceptance and selection

This opt-in evaluator pass composes the existing launch-aware, held-60 Hz
flat witnesses into one end-to-end nominal acceptance rule. It is not a
new V2 certificate, production planner, trajectory generator, controller
integration, physical-orientation migration, or robustness proof. The old
V2 classifications and all historical artifacts remain unchanged.

The question is bounded: does at least one of the frozen fifteen rows
provide a complete direct witness from source-pad rest to stable target
contact, after actual geometry and whole-flight checks, without a floor
cutaway? The pass stops after that flat gate.

## Frozen inputs and no-physics preflight

Use the sealed inputs from
[`waypoint_direct_paired_command_feasibility_protocol.md`](waypoint_direct_paired_command_feasibility_protocol.md).
Rebuild the upstream input gates and the three-by-five source-duration
family before constructing a simulation state. Bind the paired-command
summary to semantic identity `fnv1a64:080e8e9867b02d97`, recomputing the
identity rather than trusting its stored value. Its accepted `run_d` and
`run_e` files have SHA-256
`6477a947537208872056d46e82c7b6fc0c7aa6801f14ca9ccb8b340f4da86247`.
The source-duration summary remains `fnv1a64:dfe0f0feaa15fc24`, SHA-256
`a93b728374145d4f12b43b21b7d68e383270db19facdf5964ce91bc47e557e9c`.
The runtime enforces semantic identities and bindings; raw SHA-256 values
are recorded provenance and are verified separately with `rtk sha256sum`.

Require all fifteen ordered bindings: basis identity, role, duration offset,
source tick count, source handoff, analytical membership, nested upstream
gates, and paired-command provenance. Six analytical skips remain unflown;
nine schedules remain the only executable candidates. Validate the logged
source commands' join to the full-flight trace. A missing or changed
identity/join is an invalid input, not a no-direct-witness result.

`--preflight-only` performs these checks without constructing a simulation
state, advancing physics, or creating an output directory. No solver is
rerun and no timing, command, tail, terrain, or policy is tuned.

## New complete-wrapper contract

A wrapper receives a new identity binding the exact scenario, original
basis identity, launch rule, source duration, held command schedule, coast
and terminal tail, acceptance policy, cadence, and `core_current` geometry
convention. It must not inherit the original V2 certificate's identity or
label. Paths and wall-clock values are excluded from semantic identity.

Reconstruct the candidate from its sealed launch and paired full-flight
command log. Replay at 120 Hz physics and 60 Hz held-command cadence using
the ordinary simulator and existing neutral contact seam. Require replay
parity with the sealed state/command/contact evidence, including the source
handoff and first-contact state before terminal effects erase velocity.
The authoritative core enum owns contact acceptance; an evaluator mirror
only explains signed predicate margins.

Accept a complete nominal direct witness only when all of these hold:

1. The supported initial state is upright, at rest on the source pad.
   The fixed launch completes without post-step contact and joins the
   source bridge without an attitude or state discontinuity.
2. Existing source-specific screens, strict `1e-6 m / 1e-6 m/s` handoff,
   scheduled-source-prefix parity, and core replay parity pass.
3. Every airborne post-step state through first contact passes the new
   actual-geometry terrain screen. Use core-rotated feet and hull, not
   V2's opposite-sign footprint conversion. Query the exact heightfield
   over the rotated body's world-axis envelope; a domain overrun fails.
4. A reduced-clearance pad transition is allowed only during the source
   launch/source bridge or descending terminal bridge, with both actual
   feet and the hull's horizontal bounds wholly within the corresponding
   declared flat pad. These states still require nonpenetrating actual
   rotated geometry. Outside those corridors, require the existing
   5 m airborne reserve. Center height alone never authorizes a transition.
5. Terminal entry is descending from above, with feet and hull clear of
   terrain. The frozen terminal handoff lies on the descending arc branch.
   There is no contact before the first target contact.
6. First contact is authoritative stable, safe touchdown on the target.
   At this contact use the core's existing penetration/contact tolerances,
   not the airborne clearance rule. Keep every signed foot, hull, speed,
   attitude, angular-rate, and pad-edge margin.
7. Whole-flight commandability, fuel, and time pass: no below/above throttle
   saturation, fuel exhaustion, or capped fuel-burn tick; actual consumption
   and elapsed time meet the supported limits, and planned total time meets
   the policy's mission-time reserve.

This is pointwise, discrete simulator evidence. It does not certify a
continuous swept trajectory or perturbation robustness. Existing conservative
source screens do not establish robustness of the full launch-to-contact
flight. The current core thrust/body orientation convention remains unchanged;
using it explicitly does not reconcile the physical convention with V2.

## Selection, output, and stopping gate

Retain every original row and its explicit skip/rejection reason. For each
flown wrapper retain its new identity, source endpoint errors, first failed
gate/tick, signed contact margins, minimum airborne clearance, planned time,
actual touchdown time/fuel, and replay evidence. Only after all complete
acceptance screens pass may a wrapper participate in selection. Rank by
planned total mission time, then stable wrapper identity; do not rank by
an early observed collision/contact time or select a shortest V2 seed first.

The frozen contact-only regression must reproduce four stable target contacts
(native rows 1 and 2; third-candidate rows 11 and 12) and five crashes
(research-shortest rows 5 through 9). These known outcomes are not held-out
validation. Additional geometry or whole-flight screens may reject a nominal
landing; record that rejection without relaxing the screen.

The flat gate requires all fifteen rows, the contact/replay regression,
and at least one fully accepted wrapper. Success is
`nominal_replay_validated_direct`. A valid family with zero accepted wrappers
is `unknown`, with a precise stopping result, not proof of physical
impossibility. An input/provenance mismatch fails before physics.

Write a separate create-only `summary.json`, verify serialized identity
round-trip, and require two fresh-path runs to be byte-identical. Run focused
tests, CLI parsing, workspace tests, formatting, strict Clippy, and diff
integrity on the integrated tree. Verify the old paired/source artifact hashes
remain unchanged. No commit, push, controller flight, new report server,
held-out expansion, waypoint search, or default promotion belongs to this pass.

## Reproduction

From the repository root, use the same input paths for preflight and execution:

```sh
rtk cargo run -p pd-eval -- waypoint-direct-complete-flat-acceptance \
  --paired-command-summary outputs/research/waypoint_direct_paired_command_feasibility_20260925/run_e/summary.json \
  --source-duration-summary outputs/research/waypoint_direct_source_duration_canary_20260924/run_d/summary.json \
  --baseline-summary outputs/research/waypoint_direct_contact_contract_20260924/baseline/summary.json \
  --sweep-summary outputs/research/waypoint_direct_contact_contract_20260924/sweep/summary.json \
  --nominal-summary outputs/research/waypoint_direct_contact_contract_20260924/nominal/summary.json \
  --launch-summary outputs/research/waypoint_direct_contact_contract_20260924/launch/summary.json \
  --contact-audit-summary outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json \
  --flat-canary-summary outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json \
  --coupled-audit-summary outputs/research/waypoint_direct_coupled_thrust_audit_20260924/run_a/summary.json \
  --output-dir /tmp/pd-complete-flat-preflight-only \
  --preflight-only
```

Remove `--preflight-only` and choose a fresh output path to run the pass.

## Recorded result: 2026-09-25

The frozen flat gate passes. The create-only `run_c` and `run_d` summaries in
`outputs/research/waypoint_direct_complete_flat_acceptance_20260925/` are
byte-identical, with semantic identity `fnv1a64:ba51ace2c0bf1d08` and SHA-256
`85dd1276eb3e1bcbd958cf1d50dd94fc6f82d81d1faecf20c870de61f6c06cd6`.
The earlier `run_a` has identical bytes and is retained. Preflight passed
without creating its output directory; an attempted overwrite of `run_c`
was refused.
The final integrated-tree replay `run_e`, after lint-only cleanup, also has
identical bytes.
An altered paired-summary identity was also rejected by preflight without
creating output. The original paired and source-duration SHA-256 values
were rechecked and remain unchanged.

All fifteen rows are retained: six analytical skips and nine complete
replays. All nine reproduce their frozen contact result and pass replay,
actual rotated-body clearance, source handoff, descending terminal-entry,
and whole-flight command/fuel/time checks. Native rows 1 and 2 and third-basis
rows 11 and 12 pass the full acceptance rule. Research-shortest rows 5 through
9 are rejected at their first target contact, with maximum-foot-clearance
margin about `-0.1685 m`. They do not crash at launch.

Selection is native row 1, source-duration offset `-180` ticks, complete
wrapper identity `fnv1a64:b805659d7f10768c`. Planned total time is `34.35 s`;
authoritative stable target contact occurs at `34.275 s`, using
`950.075866 kg` of fuel. Source handoff errors are `5.835864e-8 m` and
`1.133849e-9 m/s`. Both stored and independently recomputed saturation,
fuel-exhaustion, and fuel-burn-cap counters are zero. Its tightest airborne
clearance is `0.000553356 m` on the first upright launch tick, wholly inside
the source-pad transition corridor; outside pad corridors the unchanged
`5 m` reserve applies. Its stable maximum-foot-clearance margin is only
`0.0166934 m`, so nominal acceptance is not robustness evidence.

This closes the known-flat acceptance/selection question without a cutaway.
It does not close reusable generation, held-out flat/uphill/downhill coverage,
robustness, or controller/default integration. The next separately scoped
pass should reuse the launch-aware held-command construction for new inputs
and apply this same end-to-end gate before expanding to waypoint cases.
No expansion is executed by this pass.

Validation: 677 workspace tests pass across twelve suites. After the lint-only
cleanup, all seven focused module/CLI tests pass and the integrated-tree
replay remains byte-identical. Workspace formatting, strict all-target Clippy,
and `git diff --check` pass. The prior dirty worktree is preserved; no commit,
push, deployment, controller change, or default-planner change was made.
