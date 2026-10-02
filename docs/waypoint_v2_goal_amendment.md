# Waypoint V2 goal amendment

## Requested project goal

The user requests this amendment on 2026-10-01 after clarifying that nominal
construction should share a state-aware model for grounded and airborne starts,
estimate finite acquisition, and preserve already acceptable vertical motion.
The detailed work is in the [unified nominal plan](waypoint_v2_unified_nominal_plan.md).
This is a project-goal amendment, not a new measured result or permission to
implement before the research/design review is settled.

Replace the obsolete next-action choice of local-ranking versus wider fixed
airborne timing with the following objective; keep the original usable-planner
bar rather than calling the current 12/16 implementation complete:

> Advance the existing practical opt-in waypoint V2 to the declared usable
> planner gate by unifying terrain-blind nominal construction from actual pad-rest
> or airborne state. Treat the idealized arc as a minimum acceptable approach,
> not an exact profile to track: preserve already admissible vertical motion,
> correct lateral interception and only unsafe approach/energy conditions, and
> account for finite acquisition time, position, gravity, attitude/rate, fuel and
> the original deadline with a bounded inexpensive estimator.
>
> First characterize and freeze the terminal-entry envelope, estimator margins,
> candidate/ranking rules, finite budgets and supplied-command realization, then
> perform a separate primary design review. Runtime implementation needs a
> subsequent approval. Use an explicit new nominal policy; keep versions 1/2,
> local clearing acceptance/templates/ranking, production V1, core physics,
> contact/landing predicates and defaults unchanged. Do not reset state or fuel,
> restore analytical snapshots into execution, loft nominal flight to avoid
> terrain, require direct landing after every waypoint, or invent a terrain
> conflict when nominal construction is exhausted.
>
> After approved implementation, validate the complete existing loop on the same
> frozen 32 inputs: 8/8 clear zero-correction landings, at least 13/16 ordinary
> landings and 2/4 per family, a reference landing with repeated corrections,
> at least twelve initially blocked ordinary nominal proposals, full actual-state
> execution/replay integrity, six corrections and the original deadline, and
> planning median <=2 s / nearest-rank p95 <=5 s over all 24 common attempts.
> New-policy commands/handoffs may differ; preserve old modes' physical evidence
> and require new nominal independence across interior-terrain twins. Repeat
> final-source results, historical preservation and workspace test/Clippy/fmt
> checks. Do not implicitly commit, push, deploy or promote defaults. Use one
> predeclared constructor policy for the next full matrix; additional tuning
> requires another decision. Keep finite misses and the original denominator,
> and do not declare usability or completion while any required gate is unmet.

## Stored goal status

The old implementation checkpoint ended blocked at its one-revision limit and
12/16 result; that usable-planner gate remains unmet. It was subsequently cleared
outside this research pass: the goal tool returned no stored goal when the user
started the next worker loop. It was not marked complete to replace it.

The replacement goal is the bounded estimator/terminal-entry characterization
and separate primary review described in the
[research protocol](waypoint_v2_nominal_characterization_protocol.md). Its
[results](waypoint_v2_nominal_characterization_results.md) and
[review](waypoint_v2_nominal_characterization_review.md) conclude with an
evidence-backed rejection of the studied family for runtime replacement: all
39 retained airborne states have free-space witnesses, but only 2/8 clear
ground starts pass. This is a completed research/review checkpoint, not a
completed usable planner.

Closing that stored research goal does not reset the spent policy-2 revision
allowance or authorize runtime implementation. The subsequent bounded ground
diagnostic now has [results](waypoint_v2_ground_diagnostic_results.md) and a
[separate primary review](waypoint_v2_ground_diagnostic_review.md): original
durations pass at eight proven entries, the research braking-time construction
reverses lateral motion there, and flat/uphill shadow landings exceed the
unchanged thrust budget. This closes another diagnostic checkpoint, not a new
constructor or usable planner. Later runtime work still needs approval and the
unchanged full mission gate above.
