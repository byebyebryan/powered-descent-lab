# Focused mission browsing

[Presentation workflow](report_presentations.md) ·
[Development baseline](procedural_challenge_baseline.md) ·
[Documentation home](README.md)

October 11, 2026. The owner found the browsing entry ambiguous and noted that
collapsed sibling groups accumulated above the active group, separating it from
the group controls. Presentation commit `df03dac` makes the current mission group
the primary browsing surface while retaining the full mission hierarchy.

## Behavior

- **Start browsing** opens the first matching group. **Resume browsing** returns
  to the current group and page; it does not restart or advance the collection.
- **Previous group / Next group** move between terrain groups. Separate
  **Previous page / Next page** controls move within the current group. The
  selector shows the full family → morphology → band path and group position.
- Both sets of controls appear above and below the missions. The top controls
  stay visible with an opaque background while scrolling. Group changes reveal
  the active path and first missions beside the controls.
- The normal browsing view renders only the active ancestor path, preventing
  earlier sibling groups from accumulating. **Show group tree** reveals the full
  hierarchy and counts; **Focus current group** or choosing another group restores
  the compact view. The initial overview still shows family summaries.
- Coverage focus is explicit. **Show all matching groups** clears only the group
  restriction while keeping direction, outcome, approach and search filters.
- Twelve mission occurrences per page, viewport preview loading, rich details,
  physical-direction metadata and context-preserving returns remain in place.

## Current editions

| Backend | Mission review | Scheduled attempts | Rich details | Unsupported statuses |
| --- | --- | ---: | ---: | ---: |
| Layered Perlin | [Browse missions](../outputs/procedural-challenge/baseline-20261011T181000Z/layered-review-r4/report.html) | 3,290 | 2,454 | 836 |
| Value noise | [Browse missions](../outputs/procedural-challenge/baseline-20261011T181000Z/value-review-r4/report.html) | 3,332 | 2,499 | 833 |

These are create-only editions of the existing development campaign. Original
captures and earlier report editions remain retained. Controller/planner/plant,
evaluator implementation, mission generation and server lifecycle are unchanged.
The pass generated no campaign attempts or new replay evidence.

## Validation

The [preservation receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/browsing-preservation-r4.json)
binds the renderer, development gate and browser receipts. Both editions have
byte-identical `comparison-input.json` and `navigation.json` compared with the
original verified editions. All capture identities, saved rich payloads,
preview hashes, shared bounds, campaign bindings and outcome counts match.

The [layered browser receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/browser-layered-qol-r4/browser-result.json)
and [value browser receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/browser-value-qol-r4/browser-result.json)
pass at 1440 × 1000 and 390 × 844. They visit all 11 layered and 12 value groups
through the actual navigation controls, keeping the active path and first
missions in the same viewport. At most three group rows and twelve mission rows
are visible in focused browsing, including at the end of the collection. Footer
group navigation, the full-tree option, scope clearing, resume/page persistence,
sticky-control readability, direction filters, preview bounds, rich details and
return navigation pass, with no browser errors or horizontal page overflow.
Screenshots retain the last group and navigation while scrolling.

The [maintained gate log](../outputs/procedural-challenge/baseline-20261011T181000Z/browsing-development-gate.log)
records all 18 checks passing, including 85 Node tests. Source-fixture tests cover
V1 and V2 hierarchy visibility without requiring retained campaign evidence.
