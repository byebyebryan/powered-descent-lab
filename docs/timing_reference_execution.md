# Timing reference execution

This pass follows the [frozen qualification plan](timing_reference_plan.md).
Its create-only root is `outputs/timing-reference/loop-20261024T185303Z`.
The baseline binds initial head `2a87566`, 934 Git-visible file hashes and all
foreign changes. The outer deadline is Unix 1792878783; limits are one billion
charged steps, 1,200 captures, 12 GiB new artifacts, one native worker and a
40 GiB free-space floor. Fitting and RL updates remain zero.

## Initial declaration

The 63/132-pair comparison, 1% reference-selection rule, 300 independent primary
draws, 60 boundary cases, retained handoff regressions and isolated measurement
protocol precede all new campaign outcomes. No qualification inputs are generated
until the reference is frozen. Controller behavior, clocks, tolerances and old
outcomes remain fixed.

## Pilot and campaign release

Implementation commit `eac8470` adds the named 132-pair opt-in profile and
measurement sidecars. Five targeted Rust tests, strict targeted Clippy and nine
native Python tests pass, including full proof and measured/unmeasured identity.
The compiled source ledger points into the retained `source-epoch-a` copy.

The fixed pilot completed 12 Summary executions and four expanded captures with
eight reconstructions: 3,630,876 charged steps, 16,217,782 logical bytes and
2.875 s measured campaign time. All four captures verify target landing.
Maximum pilot forecast work is 14,172 / 112,016 / 200,584 steps for legacy,
63-pair and expanded selection respectively. This pilot establishes cost only.

`campaign-release.json` freezes phase caps before further outcomes: development
90 million, isolated benchmark 70 million, primary 500 million, boundary
100 million and retained regressions 100 million steps. Including the pilot's
25-million cap, reservations total 885 million within the one-billion outer
limit. Capture reservations total at most 1,165. The original three-hour,
12 GiB and free-space limits remain. Primary allocation is 1.667 million steps
per original, covering Summary, Captured, both reconstructions and neutral proof
for all three selectors. No primary inputs have been generated at release.

Development also reproduces both old profiles on all 180 old programs, rather
than checking only the 60 direct originals. This preservation check does not
expand the reference-selection or qualification denominator.
