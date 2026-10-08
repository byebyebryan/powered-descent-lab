# Procedural-terrain capability diagnostics

[Evaluation workflow](../../../docs/evaluation.md#procedural-terrain-diagnostic-pack) ·
[Selection and validation plan](../../../docs/terrain_diagnostics_plan.md) ·
[Frozen contract](../../../studies/terrain_profiles/diagnostics_plan.json)

Ten byte-identical scenarios from the inspected 1k development survey, not new
seeds or manufactured obstacle worlds. All use the tested vehicle, Earth
gravity, 120/60 Hz, a 1200 m route and original-height local pad preparation.
There is no floor cutaway. Original scenario IDs, metadata and seed identities
are intentionally preserved; filenames retain the source survey's case IDs.

| Capability | Known failing inputs | Successful comparison |
| --- | --- | --- |
| Escape rising terrain close to the pad | [327](random-327.json), [791](random-791.json) | [258](random-258.json) |
| Establish a nominal after a fast, late airborne handoff | [024](random-024.json), [516](random-516.json) | [271](random-271.json) |
| Gain useful route distance across successive corrections | [030](random-030.json), [280](random-280.json) | [253](random-253.json) |
| Preserve a clear direct landing without waypoints | none | [000](random-000.json) |

Successful comparisons share a broad symptom, not identical geometry or state.
The failures are finite search stops, not executed crashes or established
physically impossible missions. In particular, the two late-handoff cases fail
analytical screening before any new nominal physical witness. Six corrections
alone is not a defect: comparison `253` uses six and lands.

The contract pins each exact input, source capture/manifest/receipt identity,
raw baseline flight digest, full non-timing fingerprint and compact observations.
Inputs and observations are portable; collecting baseline-exact flights requires
the recorded executable. Future candidate comparison is explicitly opt-in and
must not require known failures to keep failing. No default pack, acceptance
denominator, report selection or published report page is changed by these files.

Use `waypoint-v2-flight --preflight-only` for input-only admission, or the bounded
collector in the evaluation workflow for a separately authorized capture.
Each executed mission uses the existing common rich detail report. Keep full
flight, physical outcome, mission outcome, integrity and source replay together;
an exit-zero collection or matched failure fingerprint is not a landing.
