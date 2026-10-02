# Waypoint V2 airborne integration planning review

## Verdict

The [airborne integration plan](waypoint_v2_airborne_integration_plan.md) is ready
for a separately approved bounded implementation. It targets the actual runtime
coverage gap while leaving proven startup and local-clearing behavior alone.
This is a separate primary review of the plan against current source and retained
evidence, not an independent-agent review or a claim that policy 3 exists.

The recommendation is narrower than immediately unifying both launch and
airborne actuator templates. That is an intentional simplification for the first
usable game-oriented planner, not a retreat from the terrain-blind direct-first
contract. The new research ground family remains validated evidence and can be
integrated later if useful; the existing ground path already needs no cutaway.

## Findings and resolutions

| Concern | Evidence and planned resolution |
| --- | --- |
| Could launch replacement recreate a startup or entry-spacing problem? | Runtime initial entries depend on the old source handoff and 72-tick launch offset. Research preparation/acquisition landmarks differ substantially. Keep the complete canonical initial branch and policy 2 entry formula unchanged. |
| Can powered acquisition be removed? | Nine of 39 selected airborne proposals use it; eight have no admissible zero-acquisition entry and one zero trial fails executable shape. Keep the tested family, not a coast-only shortcut. |
| Can validation work be reduced safely? | Retained ledgers need 40 attempts to reach the first valid proposal versus 114 research attempts. Stop there in fixed rank order; retain the three-attempt cap for new states. This is not a measured latency improvement. |
| Can an acquisition program masquerade as the old coast-only proposal? | The old audit infers phase from coast count and validates the old policy identity. Require a new bound complete command program and phase-aware audit using existing safety primitives, not altered legacy fields or retagging. |
| Does selection accidentally depend on terrain? | Separate free-space realization from research terrain reporting. Select the first executable proposal before actual terrain audit; do not fall back to another nominal arc after a terrain rejection. |
| Are rare backend shape failures a new research project? | One retained airborne first trial rejects shape and its next trial succeeds. Preserve finite fallthrough; do not redesign shape constraints or add a dive-and-recover solver. |
| Are cut edge cases hiding ordinary failures? | Scope remains the already tested setup and aligned forward handoffs. All retained airborne inputs fit it. Unsupported or missed ordinary missions remain in the original denominator. |
| Could simplification remove ordinary safety behavior? | Odd contact endpoints, last-consumed-command phase ownership, raw safe contact, mass/fuel/deadline and full replay remain mandatory. Source exceptions never apply to an airborne program. |
| Is another policy-revision loop implicit? | No. One named opt-in policy 3 inherits policy 2 local behavior, with one development matrix and conditional preservation/final repeats. There is no numerical revision or policy 4 budget. |
| Why not rescue only the old NoNominal states? | Legacy-first rescue would preserve more existing choices, but retain both searches and the old 56-trial cost. One airborne family is selected here; rescue remains a possible later decision if this matrix rejects replacement. |

## Readiness and acceptance boundary

The plan freezes runtime first-valid selection, bounded retries, scope exclusions,
terrain-independent ranking and existing failure types. It identifies the narrow
shared-realization extraction and requires retained payload agreement rather
than trusting a refactor. It does not load sealed answers or restore handoff
snapshots in runtime use. All actual handoffs retain their original clock, fuel,
attitude, held command and history.

The implementation must prove 39-row adapter equivalence before mission runs,
then exercise acquisition and repeated clearing in the four-case canary batch.
The complete matrix still owns usability acceptance: 8/8 clear, at least 13/16
ordinary, per-family/reference/initial-obstruction/integrity/performance gates.
Subsequent handoffs can change, so four saved recoveries cannot certify that
matrix. Finite canary coverage misses remain visible; integrity failures stop
progress rather than becoming hidden retries.

A successful matrix permits closure of the supported opt-in usability checkpoint
without fixing every diagnostic. A failed matrix permits an honest bounded-pass
rejection, not a silently weakened bar or declaration that the original usable
planner objective is complete. Default promotion and later ground unification
are not part of this approval.

Readiness uses current source inspection, 252 verified terminal-time bindings,
matching accepted summaries, the retained-state/attempt audit, frozen suite
structure, corpus/tamper, fake-suite, document-link and whitespace checks. All
1734 monitored source/script/fixture/evidence bytes preserve their fingerprint.
No new flight, code change, workspace test/Clippy run, active goal or commit is
performed. The preceding test counts remain historical, with their disclosed
baseline lint exception; planning checks do not certify new runtime coverage.
