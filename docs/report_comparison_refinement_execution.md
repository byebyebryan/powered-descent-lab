# Mission comparison refinement execution

This follows the [authorized plan](report_comparison_refinement_plan.md).
Initial head is `8a7a854`. Allocation:
`outputs/report-comparison/loop-20261024T224911Z`.
The initial workspace and both preceding complete editions are bound in
`workspace-baseline.json`; foreign work and original reports remain retained.

The allowance is four hours, 14 GiB including new publication snapshots, and a
40 GiB free-space floor. Publication is capped at 1 GiB and 16 bundles under the
existing per-bundle limits. Campaign executions, replay and training are zero.
Report services retain PIDs 37637 and 37689; no push or service change is included.

## Source and delivery preparation

The publisher currently allows 100 MiB and 2,000 files per bundle; its store has
about 12.8 GB under a 20 GiB limit. The two complete source editions contain
14,752 files and about 5.94 GB, primarily repeated standalone rich HTML.

The neutral lossless literal-column packing primitives were copied into the
owned `experiments/report-study/lossless.py`. The adoption receipt binds the
original helper hash; its separately owned untracked source is unchanged and
is not a runtime dependency. Full-inventory measurement precedes UI expansion.

The full packing feasibility pass round-trips all 1,285 timing and 2,400 ignition
rich documents exactly, including native JSON literals. Timing is 353,576,556
bytes in 114 data files; ignition is 414,502,417 bytes in 206. Each flight data
pack stays below 8 MiB. Single-bundle delivery does not fit, but same-origin
immutable data bundles fit the publication allowance without service changes.

## Shared direction inspection

Arrow geometry now places the tail on the recorded position, with a 24-pixel
full-command length. Selection spans the whole powered interval and prioritizes
late behavior and changes rather than taking only the earliest spatial points.
Mission preview cards are centered inside their cells. The shared read-only
flight model handles exact sample clocks, ended states, safe payload extraction,
angle unwrapping and paired physical camera bounds. Direction/command histories
have a shared cursor and work without Plotly; individual rich views retain all
existing tools. Nine geometry/model tests and all 62 report tests pass. Live
prototype and complete-delivery acceptance remain pending.
