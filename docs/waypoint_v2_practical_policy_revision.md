# Waypoint V2 initial entry revision

## First matrix result

The first complete policy 1 matrix on 2026-10-01 lands all eight clear controls
without corrections and eight of sixteen ordinary obstacle cases. Ridge and
plateau families each land two of four, successive obstacles land zero of four,
and sloped cases land four of four. All sixteen ordinary initial proposals are
genuinely terrain-blocked. The unchanged reference lands after three corrections,
with its original first local handoff retained. Every supported partial or
complete flight passes accumulated source and official replay.

Evidence is in
`outputs/research/waypoint_v2_practical_20261001/development_policy_1_b`.
Its 24 common attempts have planning median 0.3424 s and nearest-rank p95
0.6571 s. These are development measurements, not final repeated acceptance.
The earlier partial root `development_policy_1_a` stopped on an odd-contact
coverage-boundary adapter bug; no flight policy changed to fix that bug.

Four ordinary misses exhaust local clearing before any correction. Four stop
with NoNominal after one or more accepted corrections. These are coverage misses,
not crashes on the active flight and not physical impossibility claims. Policy 1
does not meet the declared usable-planner bar of thirteen ordinary landings and
two per family.

## One uniform revision

Before any policy 2 measurement, select the plan's single allowed revision:
replace the four initial intervention entries with source-bridge fractions
0.75, 0.625, 0.5 and 0.375. Keep the original launch offset and aligned rounding:
`E = 72 + 2 * floor(f * (S - 72) / 2)` for source handoff S. For S = 1992,
the entries are 1512, 1272, 1032 and 792 instead of 1994, 1512, 1032 and 552.

The hypothesis is that the new entry at 1272 covers the large gap between the
old 50 and 75 percent entries for earlier obstructions. Removing the late idle
entry also avoids choosing an already fast initial correction merely because
it is the latest entry. Two measured late-feature NoNominal misses start their
correction at tick 1994 and return H with horizontal speed 68.563 m/s and only
276 to 289 m left to the target. This motivates earlier entry sampling; it does
not establish that the revision will land those missions.

This is an entry-spacing revision only. Later live-origin entries, forty-two
local templates, local ranking, six-correction cap, original deadline, two-second
certificate, reserve, physics, controller and nominal generators stay unchanged.
The sealed old canary stays unchanged. The reference's original 75 percent entry
remains the latest revised entry, so its existing accepted first row stays the
local winner. No expected later row or waypoint count is supplied as an input.

Policy 1 remains the default API policy and is retained for comparison. Policy 2
has a separate policy identity and explicit CLI selection. Run the exact same
32 inputs and retain every miss in its original denominator. No further policy
revision is authorized by this implementation phase if the coverage bar is still
missed; report the actual remaining gap and request the next decision.
