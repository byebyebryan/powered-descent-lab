# Direct-leg flat third-candidate closure canary

This opt-in, outcome-known pass asks one narrow question: does the remaining
originally V2-certified flat candidate close the observed gap between the
analytical eligibility of the shortest candidate and the stable landing of
the native candidate under the already fixed launch? It is not a candidate
search, a changed V2 certificate, or a planner/controller selection rule.

## Pre-physics gate

Rebuild and check the sealed primitive baseline, topology sweep, and nominal
plant inputs, then validate the existing launch-feasibility and first-contact
audit artifacts. Pin their semantic identities, respectively:

- `fnv1a64:d3fa6b24336f7c05` (baseline)
- `fnv1a64:1bcd5a3bd6c6da01` (sweep)
- `fnv1a64:9e8cbc902ca11fbc` (nominal plant)
- `fnv1a64:2c7b965ffdc809d6` (launch feasibility)
- `fnv1a64:f5a6600e99cd283b` (first-contact audit)

Also bind the existing nominal manifest and launch/contact input-gate
identities. Before creating a simulation state, find exactly one certified
candidate `fnv1a64:a18a98ad6e334014` in the reevaluated
`continuous_flat_r00` result. The two previously flown flat identities remain
unchanged: native `fnv1a64:dee613017622ca16` and research-shortest
`fnv1a64:4e6c0b23f9eb1b8f`. Do not add the third identity to either sealed
artifact's selected roles or rewrite any prior output.

## Fixed execution and stopping rule

Materialize only this third candidate's frozen source/coast/terminal profile.
From the same source-pad-rest state, apply 60 full-throttle upright physics
ticks, followed by 12 full-throttle ticks targeting its first powered source
attitude. Re-solve only its source bridge from the achieved launch state to
the original source handoff, using the original source-bridge tick count;
keep its coast, terminal bridge, handoff, policy, vehicle, and scenario fixed.
No timing, angle, reserve, ranking, threshold, or controller search is
authorized. A finite re-solve is replayed diagnostically even if its
analytical eligibility fails; keep the analytical and plant findings
separate.

First check the 120 Hz per-tick lane's contact-free source handoff at the
declared `1e-6 m` and `1e-6 m/s` state tolerances. If this gate fails, stop
without a post-handoff or 60 Hz third-candidate rollout, and record the
specific skip. If it passes, finish that lane and independently run the
60 Hz two-tick-held command lane. For each run record re-solved analytical
classification and reserve, achieved source handoff, first-contact
pre-terminal foot/hull and safety predicates, authoritative outcome, and the
strict 60 Hz handoff diagnostic. Landing and analytical eligibility must
both hold to motivate any later selection-screen design.

Separately scan the existing 18 frozen launch traces and any newly completed
third-candidate traces, tick by tick through each original terminal event.
On the *same* pre-terminal state, compare core geometry with the opposite
footprint-sign convention used by V2. Record the first difference in
no-contact, stable-contact geometry, and pad-membership predicates, plus
whether one convention would encounter a geometric contact gate before the
other. This is a geometry-only diagnostic: commands, dynamics, and the
authoritative core classifier stay unchanged. In particular, nothing after
the original terminal tick establishes an alternate-sign flight outcome.

## Evidence and next decision

Write a fresh, identity-bound evaluator artifact under a new output
directory; no paths or clock values enter its semantic identity. Two
fresh-path runs must be byte-identical. Verify the no-physics gate, focused
tests, and the integrated workspace format, tests, strict lint, and diff
integrity gates. Preserve all historical outputs.

If the third candidate is analytically eligible, reaches the source handoff,
and lands, the next *design* step is an evaluator-only first-contact selection
screen followed by held-out flat and sloped cases. If it does not, stop this
candidate canary and separately decide whether the gap belongs to candidate
generation, the fixed launch, or the analytical policy. Neither result
authorizes lowering terrain, relaxing contact/reserve tolerances, changing
the core/V2 orientation sign, or promoting a planner, controller, F6, or
default behavior.

## Recorded result

The no-physics gate accepted the five pinned artifacts, nominal manifest
`fnv1a64:9e32aa0c1a52b28b`, launch gate
`fnv1a64:a4654d06c9166b14`, contact-audit gate
`fnv1a64:86861aa3e4f8b35e`, and the third candidate's 1,920-tick original
source bridge. Its new input-gate identity is `fnv1a64:d7476fd0e0827260`.
The 120 Hz source-only run passed the declared handoff gate with position and
velocity errors of `1.02e-12 m` and `2.56e-14 m/s`. Its complete 120 Hz
run reproduced the entire gated prefix before the independent 60 Hz run.

Both complete runs made a stable target touchdown: at physics steps 5,079
and 5,070 respectively, maximum signed foot clearance was `0.0792 m` and
`0.0879 m`, below the core `0.15 m` stable-contact limit. The pre-terminal
contact audit found both feet on the target and all stable/safe predicates
true. However, the re-solved source bridge was `NotCertified` for coupled
thrust: its normalized reserve was `0.05518`, below the unchanged `0.075`
policy requirement. The held 60 Hz lane missed strict source handoff by
`0.1774 m` and `0.02201 m/s`, even though it ultimately landed. Thus this
third candidate does **not** supply a known analytically eligible *and*
landed flat wrapper.

All 18 frozen cadence traces reproduced their pinned first-contact audit and
reached their original terminal event in the trajectory-wide sign scan.
Across those traces, the opposite-sign footprint changed pad-membership
predicates on 620 sampled states but changed no no-contact geometry gate or
stable-contact predicate. The third candidate similarly had 29 pad-membership
differences per cadence, no no-contact/stable-predicate difference, and both
conventions reached the geometry gate on the same original terminal sample.
These pad-membership differences occurred on states with original contact
classification `none`; they do not establish a different flight outcome.

Two fresh-path summaries at
`/tmp/pd-flat-candidate-closure-run-a.vjfTMD/summary.json` and
`/tmp/pd-flat-candidate-closure-run-b.3P3J04/summary.json` were
byte-identical, SHA-256
`18898544e49f2b8452e47ab4d206761a03ec54602a2a4b429e6c244e2e90bba1`,
semantic identity `fnv1a64:f3fba9290c9073be`. A third run at
`outputs/research/waypoint_direct_flat_candidate_closure_20260924/canary/summary.json`
has the same hash. The old launch-feasibility and first-contact commands
were also rerun into fresh output directories: their summaries matched the
sealed SHA-256 values `deab83d3af4cb96b0c608a1266912fc83c1d96bdba750b7610470e74b0627174`
and `35021d0a7e4f13ee4cb3c1ba8c31be95592df494dd1688a5bcad58b3b16bdfaa`.
Focused tests passed, as did the integrated 638-test workspace suite,
format, strict Clippy, and diff integrity.

The canary stops here. All three originally certified flat candidates have
now been flown under this fixed launch: the native and third landed but
their re-solved source bridges failed the analytical reserve, while the
research-shortest was analytically eligible but crashed at first contact.
This is a candidate-generation/launch/policy decision, not a reason to relax
the contact rule or robustness threshold. A next research pass should first
identify which constraint is intended to be held fixed, then predeclare a
single bounded experiment. The orientation convention also remains a
cross-model semantic issue, but this trajectory scan does not implicate it
in the observed contact outcomes.
