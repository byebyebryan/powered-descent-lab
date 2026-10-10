//! Local command protection, not waypoint placement or terminal guidance.
use super::*;

pub(super) const MAX_COMMANDS: usize = 4;
pub(super) const MAX_RECOVERY_TICKS: u64 = 600;
pub(super) const BRAKING_TILT: f64 = std::f64::consts::PI / 6.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainCorrection {
    pub episode_start_physics_step: u64,
    pub rejected_command: Command,
    pub selected_command: Command,
    pub selected_choice: String,
    pub prediction_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryCommandQuery {
    pub choice: String,
    pub command: Command,
    pub prediction_ticks: u64,
    pub checked: bool,
    pub conflict: Option<PredictedConflict>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryQuery {
    pub origin: SimulationStateSnapshotV1,
    pub commands: Vec<RecoveryCommandQuery>,
}

pub(super) fn choices(
    s: &SimulationState,
    requested: Command,
) -> [(&'static str, f64); MAX_COMMANDS] {
    [
        ("support_requested_turn", requested.target_attitude_rad),
        (
            "support_current_attitude",
            s.attitude_rad.sin().atan2(s.attitude_rad.cos()),
        ),
        ("upright_lift", 0.0),
        ("braking_lift", -s.velocity_mps.x.signum() * BRAKING_TILT),
    ]
}

/// A response-room estimate, not a trajectory or landing feasibility claim.
pub(super) fn response_ticks(ctx: &RunContext, s: &SimulationState, command: Command) -> u64 {
    let delta = (command.target_attitude_rad - s.attitude_rad)
        .sin()
        .atan2((command.target_attitude_rad - s.attitude_rad).cos())
        .abs();
    let turn = (delta / (ctx.vehicle.max_rotation_rate_radps * ctx.sim.physics_dt_s() * 2.0)).ceil()
        as u64
        * 2;
    let remaining = ((ctx.sim.max_time_s / ctx.sim.physics_dt_s()).ceil() as u64)
        .saturating_sub(s.physics_step + 2);
    (turn + REFRESH_TICKS)
        .min(CONTINUATION_TICKS)
        .min(remaining)
}

/// Warning room for the same finite command family, not a new trajectory search.
pub(super) fn warning_ticks(ctx: &RunContext, s: &SimulationState, requested: Command) -> u64 {
    choices(s, requested)
        .into_iter()
        .map(|(_, angle)| {
            response_ticks(
                ctx,
                s,
                Command {
                    throttle_frac: 1.0,
                    target_attitude_rad: angle,
                },
            )
        })
        .max()
        .unwrap_or(REFRESH_TICKS)
}

/// Every choice uses the ordinary plant. Never edits the state, goal or clock.
#[cfg(test)]
pub(super) fn select(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    requested: Command,
    episode_start: u64,
) -> Result<Option<TerrainCorrection>> {
    Ok(select_with_evidence(request, ctx, s, requested, episode_start)?.0)
}

pub(super) fn select_with_evidence(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    requested: Command,
    episode_start: u64,
) -> Result<(Option<TerrainCorrection>, RecoveryQuery)> {
    let mut query = RecoveryQuery {
        origin: SimulationStateSnapshotV1::from_state(s),
        commands: Vec::new(),
    };
    let mut seen = Vec::with_capacity(MAX_COMMANDS);
    for (name, angle) in choices(s, requested) {
        let command = Command {
            throttle_frac: 1.0,
            target_attitude_rad: angle,
        };
        if seen.contains(&command) {
            continue;
        }
        seen.push(command);
        let ticks = response_ticks(ctx, s, command);
        let mut evidence = RecoveryCommandQuery {
            choice: name.into(),
            command,
            prediction_ticks: ticks,
            checked: false,
            conflict: None,
        };
        if ticks < REFRESH_TICKS {
            query.commands.push(evidence);
            return Ok((None, query));
        }
        evidence.checked = true;
        evidence.conflict = short_conflict_ticks(request, ctx, s, command, false, ticks)?;
        let safe = evidence.conflict.is_none();
        query.commands.push(evidence);
        if safe {
            return Ok((
                Some(TerrainCorrection {
                    episode_start_physics_step: episode_start,
                    rejected_command: requested,
                    selected_command: command,
                    selected_choice: name.into(),
                    prediction_ticks: ticks,
                }),
                query,
            ));
        }
    }
    Ok((None, query))
}

/// Preserve choice order, but do not accept a shorter choice-specific proof.
/// The comparison is native, at the same actual origin and common horizon.
pub(super) fn select_common(
    comparison: &phase_transition::RecoveryComparison,
    episode_start: u64,
) -> (Option<TerrainCorrection>, RecoveryQuery) {
    let mut query = RecoveryQuery {
        origin: comparison.origin.clone(),
        commands: Vec::new(),
    };
    for choice in &comparison.commands {
        query.commands.push(choice.clone());
        if choice.checked && choice.conflict.is_none() {
            return (
                Some(TerrainCorrection {
                    episode_start_physics_step: episode_start,
                    rejected_command: comparison.requested_command,
                    selected_command: choice.command,
                    selected_choice: choice.choice.clone(),
                    prediction_ticks: comparison.prediction_ticks,
                }),
                query,
            );
        }
    }
    (None, query)
}
