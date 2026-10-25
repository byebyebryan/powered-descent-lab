# Compact mission-tree refinement

The user requested a second Luna loop after reviewing the inline comparison.
Design, layout, prototypes and browser acceptance belong to the primary. Luna
may implement a settled contract; it does not decide the layout or information
hierarchy. Initial head is `c4c83b7`.

## Purpose and identity

The tree must make neighbouring cases easy to scan, as well as comparing variants
within a case. Existing terminal/transfer scenario cells still contain seeded
physical inputs: their canonical smoke/full schedules use three/twelve seeds per
controller lane. Their aggregate previews, mean/spread, seed rows and selectors
remain authoritative. Current timing/RL studies instead browse resolved inputs
within declared families/conditions. Do not invent nominal scenarios by rounding
states, merge independent inputs into repeated executions, or treat repeated cost
measurements as additional reliability cases.

Reporting distinguishes family/group, scenario/cell when declared, resolved
seed/input and execution/variant. This pass preserves native persisted identities
and all scientific grouping and pairing. It changes presentation only.

## Primary layout decision

The current timing leaf is 451 px high at 1440x1000. A primary browser prototype
puts the same two previews beside a compact results table and measures about
139 px, with five adjacent cases visible below navigation. Use this horizontal
composition for the maintained view, without shrinking or clipping text to meet
a hard height. The desktop acceptance target is 120-150 px for ordinary closed
leaves at 1440x1000; sparse/long/unavailable content may grow when necessary.

- Keep the outer identity column and a second content column. Inside content,
  place a 324 px two-preview strip beside flexible results. Previews retain their
  156x92 aspect, exact saved frame, full trajectory and source image semantics.
- Show A/B tags on comparison previews and metric rows. Declare their full names
  once in the active group's navigation legend; retain accessible full labels on
  each preview/row. A single report uses one preview and a short saved-run label.
- Show outcome once per run, in the results table. Do not repeat the outcome in
  preview headings and prose. Use compact field headings with units and complete
  source labels available through accessible labels/titles.
- Show at most three overview measurements. Study views retain the selected
  metric and available coast/burn fields. Non-study typed views show flight time,
  fuel remaining and endpoint speed when available. Keep exact saved values and
  the existing formatting precision; exact-zero differences render as zero.
- A short result line retains the transition and eligible selected-metric delta.
  Failed/missing outcomes cannot become fuel improvements. Keep pinned earlier
  references and their same-capture/no-change disclosure visible.
- Put all measurements, original captions, detailed target context, provenance
  and legacy summaries in mission expansion. Retain all attempt links and the
  on-demand exact-time inspector. A static single campaign uses a native details
  disclosure for the additional information.
- On phones keep the two small previews together where readable, with a compact
  table beneath them. Do not expand every value into a separate form-style block.
  At intermediate widths allow results to move below previews; no clipped tables
  or horizontal overflow. Single/comparison layouts retain the same vocabulary.
- Do not force one global camera across unlike missions. Each A/B pair keeps its
  saved shared frame, consistent display size and visible physical scale. Older
  seeded aggregate previews retain all their source traces and seed spread.
- Study family/group rows retain their complete matched-input outcome counts and
  add selected-metric paired mean delta, min/max delta and eligible count. These
  are descriptive current-selection values from eligible pairs, not differences
  between unmatched success means or replacements for frozen conclusions.

## Ownership and implementation boundary

Primary owns this design, `campaign_view.css`, browser prototypes/checks, portable
edition integration, documentation, publication, review and final acceptance.
Luna's optional implementation slice owns `mission_presentation.js`, the markup
and legend integration in `paired_review.js`, the static single-run markup in
`campaign_view.rs` and focused semantic/rendering tests. Source/template changes
must preserve payloads, evidence, navigation and lazy loading. No worker may
change CSS or choose a different layout. Return unresolved decisions to primary.

Existing seeded terminal/transfer aggregation already provides the right case
summary and drill-down contract. Review it against the compact style; do not
replace it with one selected seed or introduce a new aggregate authority.

## Validation and delivery

Use complete retained timing/RL inventories, genuine single/non-study comparison
renderers and saved terminal/transfer aggregate reports. Check ordinary, missing,
failed, tied and same-capture references; all 445 timing occurrences and 600 RL
inputs; all 189 terminal and 297 transfer smoke records; seed counts and aggregate
trace membership; detail/navigation and exact inspector clocks. Test desktop,
intermediate and phone widths, decoded previews, measured row heights, several
neighbouring cases, no initial full-flight packs and disposal of inspection.

Freeze production sources only after browser corrections, then run the maintained
nineteen-step development gate. Commit owned work locally. Create new small UI
editions through the existing publisher, reusing existing timing/RL data aliases.
Saved seeded batch pages may keep explicitly labelled archive detail links on
8000; that is not a migration or republication of their flight captures. Preserve
all prior reports, source captures, receipts and foreign dirty/untracked work.

Allocation is `outputs/report-compact-tree/loop-20261025T170217Z`. Its baseline
binds 994 workspace files and 18,786 retained files, including the previous loop
and saved seeded batches. Enforce 2 GiB new artifacts including snapshots, four
new publications / 64 MiB, and a 40 GiB free-space floor. No new data publications,
flight campaign, source replay, training, push, deletion or service/configuration
change. Do not start a second server or overwrite an old report edition.
