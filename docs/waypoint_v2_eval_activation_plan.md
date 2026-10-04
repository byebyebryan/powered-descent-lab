# Planner V2 evaluation activation

## Scope and exit gate

The accepted policy-3 piecewise ballistic planner becomes the primary planner
evaluation workflow. This is an integration and reporting pass, not another
trajectory research pass. The user authorized implementation and reviewed
incremental commits. Ordinary `pd-cli run`, controller defaults, plant/contact
rules, numerical parameters, and planner search/selection remain unchanged.

The exit gate is a default planner batch command that runs the existing 32-case
practical suite plus the existing 12-case additional terrain suite, produces
truthful grouped results and rich mission reports with executed handoffs, and
is discoverable from the report home without a supplied deep link. Existing
physical inputs and accepted capture bytes must be preserved.

## Implementation contract

1. Add a named `planner_v2_lab_suite` pack referencing the two tracked input
   suites. Resolve their exact physical scenarios from tracked fixtures, not
   archived flight answers. Verify expansion against existing construction
   and accepted scenarios before flight validation.
2. Make the normal batch CLI accept this pack and select it when no pack is
   supplied. Make the public V2 single-flight command select policy 3 by default,
   retaining explicit policy 1/2 selection. Preserve historical policy identity
   constructors and frozen research protocols.
3. Keep native V2 batch records distinct from legacy controller-run records.
   Do not forge controller specifications, V1 route contracts, or normal batch
   cache/comparison evidence. Unsupported comparison/cache options must be
   explicit, not silently reinterpreted.
4. Each execution owns a fresh capture directory. Keep the capture's pack,
   scenario, full flight, compact summary, and source/input provenance. Publish
   a current batch view only after all cases and report generation complete.
   Failed/incomplete captures must not replace the current report.
5. Report clear/direct, ordinary terrain, additional terrain, and diagnostic
   groups separately. Ordinary misses remain in their denominator. A finite
   planner stop is not a crash or a successful landing; unsupported inputs are
   not simulated. Landing requires target touchdown, mission success, landed
   planning stop, integrity, and final-source replay evidence.
6. Preserve the existing detailed report's plots and data. Add executed
   waypoint-entry/handoff states, correction reasons, replanning explanations,
   and home/topic/batch/previous/next navigation to all simulated case reports.
   A preflight-only unsupported case receives an honest diagnostic page, not
   fabricated trajectory data.
7. Organize navigation as Reports -> Waypoint planning -> Current V2 batch ->
   Mission detail. Classify V1 planner packs and old V2 presentation editions as
   legacy/history; preserve their links and original evidence.

## Review and validation

- Unit tests for input expansion and equivalent scenario/pad values, policy
  selection, typed outcome counting, capture protection, diagnostics, and
  report-only regeneration.
- CLI tests for default and explicit packs/policies and unsupported options.
- Navigation tests for root/topic/batch/detail traversal and missing captures.
- One new policy-3 44-case batch; compare scenarios, program/segment evidence,
  outcomes, correction counts, and replay/integrity to accepted captures.
  Timing/provenance fields are not trajectory differences. No case replacement
  or numerical tuning is authorized by this integration pass.
- Verify rich-report payload preservation beyond additive annotations and check
  report links/desktop/mobile behavior on the existing server if available.
- Final workspace tests, formatting, and Clippy with the existing narrowly
  documented lint allowance; focused JavaScript and retained report gates as
  relevant. Review and commit cohesive accepted slices; no push or server
  restart.

## Boundaries

This activation establishes the default **planner evaluation/report** workflow
for the supported vehicle, Earth gravity, and 120/60 Hz setup. It does not
establish arbitrary vehicle/gravity coverage, hard diagnostic success, live
game integration, or a V2 `pd-cli` controller adapter. Legacy terminal and
transfer packs continue to use their existing controller evaluation pipeline.
