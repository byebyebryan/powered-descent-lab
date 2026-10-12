# Unified procedural mission review

[Presentation workflow](report_presentations.md) · [Development baseline](procedural_challenge_baseline.md) · [Documentation home](README.md)

October 11, 2026. The owner requested one browsing report with Layered Perlin
and Value noise as top-level mission families, retaining independent statistics
and provenance. Presentation commit `de879b3` extends the maintained comparison
renderer and report discovery to provide that view.

## Current report

[Open the unified mission review](../outputs/procedural-challenge/baseline-20261011T181000Z/combined-review-r2/report.html).
It is also listed under **Complete procedural terrain** in the report library.

| Terrain family | Mission occurrences | Browsing groups | Reflected incumbent verified landings |
| --- | ---: | ---: | ---: |
| Layered Perlin | 1,645 | 11 | 1,266/1,645 (77.0%) |
| Value noise | 1,666 | 12 | 1,290/1,666 (77.4%) |

The tree follows family → measured morphology → chord-barrier band → mission →
controller attempts. It contains all 3,311 physical inputs and 6,622 scheduled
raw/reflected attempts from the same existing development campaign. Backend
rates and outcome bars remain separate; clicking an outcome restricts review to
that family. Schedule accounting and provenance retain both source populations.

Full-tree and focused-group browsing retain the previous view-selection and
local-collapse fixes. Previous/next group crosses the family boundary, with
twelve missions per page and viewport preview loading. Initial summaries load
no mission images. Rich detail links return to this unified report and preserve
view, filters, selected mission and page; matched previous/next details can also
cross between families.

The edition has 4,953 rich native details, 1,669 unsupported status details and
four common campaign companions. The earlier per-backend reports and original
campaigns remain retained. This is a presentation of exposed development inputs,
with no new campaign flights, replay evidence, benchmark freeze or promotion.

## Validation

The [preservation receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/combined-preservation-r2.json)
binds the source checkpoint, renderer, report, navigation, browser receipt and
development gate. Every campaign and flight projection matches the r5 reports
after removing only presentation link paths. Every original evaluation seal,
saved rich payload, preview hash, shared world bounds and capture identity
matches by physical source and approach. Independent-verification and source
projection hashes match both original populations, and their outcomes remain
unchanged. New combined mission anchors have their own namespace; old report
selectors remain in their retained editions.

The [browser receipt](../outputs/procedural-challenge/baseline-20261011T181000Z/browser-combined-r2/browser-result.json)
passes at 1440 × 1000 and 390 × 844 with 18 screenshots and no browser errors.
It visits all 23 groups, verifies separate backend counts and outcome filtering,
checks both tree views and local collapse, and follows a rich detail from the
last Layered Perlin occurrence into the first Value noise occurrence and back
to unified review. Focused navigation retains at most three group rows and
twelve mission rows. The combined report is visible in the procedural library.

The [development gate](../outputs/procedural-challenge/baseline-20261011T181000Z/combined-development-gate-r2.log)
passes all 18 checks, including 86 Node tests and strict Clippy. Source fixtures
cover disjoint population composition, repeat retention, reordered approach
labels, descriptor/identity rejection and bounded report discovery. All five
report-server Python tests pass, including discovery of a new challenging
collection without scanning its details. Navigation-only refresh updates the
library; server lifecycle and accepted report bodies remain unchanged.

The r1 inspection edition and the first gate's corrected Clippy failure remain
retained as intermediate artifacts; r2 and its linked receipts are the final
validated edition.
