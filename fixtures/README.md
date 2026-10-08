# Fixture ownership

[Evaluation workflow](../docs/evaluation.md) · [Current guidance](../docs/guidance.md) · [Research archive](../docs/history.md)

Fixtures are source-controlled inputs and identity-bound records, not generated
captures. A filename containing `research`, `v1`, `experimental` or `frontier`
does not by itself tell you whether an input is current or executable.

| Directory | What it contains |
| --- | --- |
| `packs/` | Current controller matrices, the native V2 pack, explicit comparison fixtures and deliberately retained retired-pack metadata. |
| `scenarios/` | Authored concrete scenarios; family/seed expansion belongs to the evaluator. |
| `research/` | Frozen physical inputs and plans. Some are current pack dependencies; others belong to closed experiments. |
| `evidence/` | Retained development/diagnostic inputs, not a substitute for a fresh capture. |
| `manifests/` | Frozen source/input/prediction/result identities for earlier bounded research. |
| `reports/` | Maintained scorecard catalog, topic hierarchy and explicitly selected historical presentation previews. |

The default planner [pack](packs/planner_v2_lab_suite.json) pins two source files
by SHA-256:

- [Practical suite](research/waypoint_v2_practical_suite_plan_v1.json): 32 cases.
- [Fresh-terrain inputs](research/waypoint_v2_fresh_terrain_inputs_v1.json): 12 cases.

Together they provide 36 core cases and eight separate diagnostics. These
`research/` files are active dependencies despite their historical filenames.
Do not edit them to make a cleanup or acceptance check pass. A new terrain
campaign needs separately declared inputs and evidence; it is not a fixture
rename or an overwrite of the accepted pack.

The separate [procedural-terrain diagnostics](research/terrain_diagnostics_v1/README.md)
freeze ten exact worlds from the inspected 1k population: six known finite stops
and four successful comparisons. They are opt-in capability tests, not additional
default-pack cases or a held-out coverage denominator. Their source provenance
and baseline observations live in the separate study contract.

Eleven old V1/generated-route, comparison-scorer, dogleg and late-bend packs
remain archived metadata. The evaluator's
[admission rules](../pd-eval/src/resolution.rs) reject their execution, including
retired profiles/controllers reused through renamed packs. Keeping metadata
allows old descriptors and reports to remain intelligible without restoring
failed experiments. Ordinary terminal, direct-transfer and authored-waypoint
packs remain separate from native planner acceptance.

`reports/report_navigation.json` describes navigation, not flight acceptance.
`reports/navigation_preview.json` selects a historical edition, not the current
native capture. The accepted selector and generated pages live under ignored
`outputs/`; navigation changes must not silently promote different evidence.
See [report operations](../docs/reports.md) before refreshing them.

The planner crate also retains [its own fixtures](../pd-plan/fixtures) for
identity-bound tests. Do not delete frozen inputs or result manifests just
because their executable experiment was retired. Never put regenerated
captures, caches, secrets or rendered report pages into fixture directories.
