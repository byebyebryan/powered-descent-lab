//! Geometry-only repair of rejected waypoint proposals. Never changes the plant,
//! the terrain-blind constructor, command protection or actual H admission.
use super::*;

// Upward roundoff allowance, not a reduction of the physical clearance reserve.
pub(super) const NUMERIC_HEIGHT_MARGIN_M: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointHeightRepair {
    pub from_goal: Goal,
    pub height_increase_m: f64,
    pub limiting_conflict: PredictedConflict,
    pub checked_through_relative_tick: u64,
    pub handoff_height_tolerance_m: f64,
}

pub(super) fn projected_view(
    ctx: &RunContext,
    source: &SimulationState,
    arc: &BallisticAim,
    tick: u64,
) -> SimulationState {
    let projected = aim::project(
        KinematicStateV2 {
            position_m: source.position_m,
            velocity_mps: arc.departure_velocity_mps,
        },
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        tick,
    );
    let mut view = source.clone();
    view.position_m = if tick == arc.steps {
        arc.target_m
    } else {
        projected.position_m
    };
    view.velocity_mps = projected.velocity_mps;
    view.attitude_rad = 0.0;
    view.physics_step += tick;
    view.sim_time_s += tick as f64 * ctx.sim.physics_dt_s();
    view
}

pub(super) fn continuation_conflict(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    source: &SimulationState,
    goal: &Goal,
    arc: &BallisticAim,
) -> Result<Option<PredictedConflict>> {
    let through = arc
        .steps
        .checked_add(CONTINUATION_TICKS)
        .context("waypoint query clock overflow")?;
    for tick in arc.steps + 1..=through {
        let mut view = projected_view(ctx, source, arc, tick);
        // Actual H may be admitted this far below the geometric waypoint.
        // Account for that existing window; do not change its admission rule.
        view.position_m.y -= tolerance(ctx, goal);
        if let Some(conflict) = reserve_conflict(
            request,
            ctx,
            &view,
            false,
            "waypoint_ideal_continuation_reserve",
        )? {
            return Ok(Some(conflict));
        }
    }
    Ok(None)
}

/// At a fixed flight duration N, raising the target by dy raises the ideal arc
/// at relative tick k by dy*k/N, including its short unpowered continuation.
/// Scan the complete body-envelope query, not only its first warning, and lift
/// the waypoint enough for the largest scaled deficit. Reconstruction may pick
/// another duration, so callers must recheck it; this is not flight admission.
pub(super) fn height_repair(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    source: &SimulationState,
    goal: &Goal,
    arc: &BallisticAim,
) -> Result<Option<WaypointHeightRepair>> {
    ensure!(
        !goal.destination && arc.steps > 0 && arc.target_m == goal.position_m,
        "height repair requires a waypoint arc"
    );
    let through = arc
        .steps
        .checked_add(CONTINUATION_TICKS)
        .context("waypoint query clock overflow")?;
    let mut worst: Option<(f64, PredictedConflict)> = None;
    for tick in 1..=through {
        let view = projected_view(ctx, source, arc, tick);
        let (clearance, required, _) = body_clearance(request, ctx, &view, false)?;
        let handoff_allowance = if tick >= arc.steps {
            tolerance(ctx, goal)
        } else {
            0.0
        };
        let lift = (required + handoff_allowance - clearance) * arc.steps as f64 / tick as f64;
        if lift > 0.0 && worst.as_ref().is_none_or(|(previous, _)| lift > *previous) {
            worst = Some((
                lift,
                PredictedConflict {
                    state: SimulationStateSnapshotV1::from_state(&view),
                    cause: if tick <= arc.steps {
                        "waypoint_incoming_body_reserve".into()
                    } else {
                        "waypoint_ideal_continuation_reserve".into()
                    },
                },
            ));
        }
    }
    Ok(worst.map(
        |(height_increase_m, limiting_conflict)| WaypointHeightRepair {
            from_goal: goal.clone(),
            height_increase_m: height_increase_m + NUMERIC_HEIGHT_MARGIN_M,
            limiting_conflict,
            checked_through_relative_tick: through,
            handoff_height_tolerance_m: tolerance(ctx, goal),
        },
    ))
}
