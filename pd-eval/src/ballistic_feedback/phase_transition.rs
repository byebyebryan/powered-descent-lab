//! Bounded native transition queries. Clones are never substituted for the plant.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub kind: String,
    pub origin: SimulationStateSnapshotV1,
    pub prediction_ticks: u64,
    pub checked_ticks: u64,
    pub end_state: Option<SimulationStateSnapshotV1>,
    pub conflict: Option<PredictedConflict>,
    pub domain_stop: Option<TerrainDomainStop>,
    pub rejection: Option<String>,
    pub first_frame: Option<pd_control::ControllerFrame>,
}

impl Query {
    pub(super) fn accepted(&self) -> bool {
        self.rejection.is_none() && self.conflict.is_none() && self.domain_stop.is_none()
    }

    fn new(kind: &str, s: &SimulationState, ticks: u64) -> Self {
        Self {
            kind: kind.into(),
            origin: SimulationStateSnapshotV1::from_state(s),
            prediction_ticks: ticks,
            checked_ticks: 0,
            end_state: None,
            conflict: None,
            domain_stop: None,
            rejection: None,
            first_frame: None,
        }
    }

    fn guard(&mut self, result: Result<Option<PredictedConflict>>) -> Result<bool> {
        match result {
            Ok(Some(conflict)) => {
                self.conflict = Some(conflict);
                self.rejection = Some("native_guard".into());
                Ok(false)
            }
            Ok(None) => Ok(true),
            Err(error) => {
                if let Some(domain) = error.downcast_ref::<TerrainDomainStop>() {
                    self.domain_stop = Some(domain.clone());
                    self.rejection = Some("prediction_domain".into());
                    Ok(false)
                } else {
                    Err(error)
                }
            }
        }
    }
}

pub(super) fn coast_settling(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
) -> Result<Query> {
    let ticks = avoidance::response_ticks(ctx, s, Command::default());
    let mut evidence = Query::new("actual_idle_rotation", s, ticks);
    if ticks < REFRESH_TICKS {
        evidence.rejection = Some("insufficient_response_budget".into());
        return Ok(evidence);
    }
    if !evidence.guard(short_conflict_ticks(
        request,
        ctx,
        s,
        Command::default(),
        false,
        ticks,
    ))? {
        return Ok(evidence);
    }
    let mut query = s.clone();
    query.set_command(Command::default());
    for _ in 0..ticks {
        query.step_with_contact_report(ctx);
    }
    evidence.checked_ticks = ticks;
    evidence.end_state = Some(SimulationStateSnapshotV1::from_state(&query));
    Ok(evidence)
}

/// Same configured controller, adapter, paired plant and guards as live entry.
/// A clear prefix is not a certificate for the remaining landing.
pub(super) fn terminal_takeover(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    terminal: &TerminalPdgController,
    deadline: u64,
    experiment: WaypointExperiment,
) -> Result<Query> {
    let mut evidence = Query::new("configured_terminal_prefix", s, CONTINUATION_TICKS);
    let mut query = s.clone();
    let mut controller = terminal.clone();
    for _ in 0..CONTINUATION_TICKS / 2 {
        if query.physics_step + REFRESH_TICKS >= deadline || query.fuel_kg <= 0.0 {
            evidence.rejection = Some("original_budget".into());
            break;
        }
        let mut frame = controller.update(ctx, &query.build_observation(ctx));
        if experiment.pad_clearance()
            && let Err(error) = protect_pad(request, ctx, &query, &mut frame)
        {
            evidence.guard(Err(error))?;
            break;
        }
        if evidence.first_frame.is_none() {
            evidence.first_frame = Some(frame.clone());
        }
        if !evidence.guard(short_conflict(request, ctx, &query, frame.command, true))? {
            break;
        }
        query.set_command(frame.command);
        for _ in 0..2 {
            query.step_with_contact_report(ctx);
            ensure!(snapshot_finite(&query), "nonfinite terminal prefix");
            evidence.checked_ticks += 1;
            if query.is_terminal() {
                break;
            }
        }
        if query.is_terminal() {
            ensure!(
                query.physical_outcome == PhysicalOutcome::LandedOnTarget
                    && query.mission_outcome == MissionOutcome::Success,
                "guarded terminal prefix contacted terrain"
            );
            break;
        }
    }
    evidence.end_state = Some(SimulationStateSnapshotV1::from_state(&query));
    Ok(evidence)
}

/// Reserve-consistent recentering adapter, not a relaxed landing corridor.
/// Only a native-confirmed outside-pad reserve conflict enables it. Keep the
/// nominal lateral acceleration, allocate remaining authority to lift, and
/// issue the replacement only if the ordinary native guard actually passes.
pub(super) fn protect_pad(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    frame: &mut pd_control::ControllerFrame,
) -> Result<()> {
    let Some(conflict) = short_conflict(request, ctx, s, frame.command, true)? else {
        return Ok(());
    };
    if conflict.cause != "short_command_reserve" {
        return Ok(());
    }
    // The native reserve query selected its 5 m floor at the predicted state;
    // an inside-pad zero-floor contact/clearance failure is not recentering.
    let predicted = nominal_body_reserve_query(
        ctx,
        request,
        &state_at_conflict(ctx, s, frame.command, conflict.state.physics_step)?,
        "terminal_bridge",
        false,
    )?;
    if predicted.1 < 5.0 {
        return Ok(());
    }
    let max_accel = ctx.vehicle.max_thrust_n / s.mass_kg(ctx);
    let raw = frame.command.throttle_frac.clamp(0.0, 1.0);
    let applied = if raw == 0.0 {
        0.0
    } else {
        ctx.vehicle.min_throttle_frac + raw * (1.0 - ctx.vehicle.min_throttle_frac)
    };
    let ax = max_accel * applied * frame.command.target_attitude_rad.sin();
    let ay = (max_accel * max_accel - ax * ax).max(0.0).sqrt();
    let replacement = Command {
        throttle_frac: 1.0,
        target_attitude_rad: ax.atan2(ay),
    };
    if short_conflict(request, ctx, s, replacement, true)?.is_none() {
        frame
            .metrics
            .insert("guidance.pad_reserve_lift_adapter".into(), true.into());
        frame.metrics.insert(
            "guidance.pad_reserve_original_throttle".into(),
            frame.command.throttle_frac.into(),
        );
        frame.command = replacement;
        frame.status = "terminal pad reserve lift while recentering".into();
    }
    Ok(())
}

fn state_at_conflict(
    ctx: &RunContext,
    s: &SimulationState,
    command: Command,
    tick: u64,
) -> Result<SimulationState> {
    ensure!(
        tick >= s.physics_step && tick <= s.physics_step + REFRESH_TICKS,
        "pad query clock differs"
    );
    let mut query = s.clone();
    query.set_command(command);
    while query.physics_step < tick {
        query.step_with_contact_report(ctx);
    }
    Ok(query)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryComparison {
    pub origin: SimulationStateSnapshotV1,
    pub requested_command: Command,
    pub prediction_ticks: u64,
    pub commands: Vec<avoidance::RecoveryCommandQuery>,
    pub queued_program: Query,
}

pub(super) fn recovery_comparison(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    requested: Command,
    correction: Option<&Correction>,
) -> Result<RecoveryComparison> {
    let ticks = avoidance::warning_ticks(ctx, s, requested);
    let mut commands = Vec::new();
    for (name, angle) in avoidance::choices(s, requested) {
        let command = Command {
            throttle_frac: 1.0,
            target_attitude_rad: angle,
        };
        if commands
            .iter()
            .any(|q: &avoidance::RecoveryCommandQuery| q.command == command)
        {
            continue;
        }
        // Domain overruns in a diagnostic alternative decline that query only.
        // They must not terminate an otherwise valid ordinary flight.
        let conflict = match short_conflict_ticks(request, ctx, s, command, false, ticks) {
            Ok(conflict) => conflict,
            Err(error) if error.downcast_ref::<TerrainDomainStop>().is_some() => {
                let domain = error.downcast_ref::<TerrainDomainStop>().unwrap();
                Some(PredictedConflict {
                    state: domain.query_state.clone(),
                    cause: "diagnostic_prediction_domain".into(),
                })
            }
            Err(error) => return Err(error),
        };
        commands.push(avoidance::RecoveryCommandQuery {
            choice: name.into(),
            command,
            prediction_ticks: ticks,
            checked: true,
            conflict,
        });
    }
    let mut queued = Query::new("queued_turn_burn_coast", s, ticks);
    let mut query = s.clone();
    for _ in 0..ticks / 2 {
        let command = if let Some(plan) = correction {
            if query.physics_step < plan.burn_end_physics_step {
                correction_command(ctx, &query, plan)?
            } else {
                Command::default()
            }
        } else {
            requested
        };
        if !queued.guard(short_conflict(request, ctx, &query, command, false))? {
            break;
        }
        query.set_command(command);
        for _ in 0..2 {
            query.step_with_contact_report(ctx);
        }
        queued.checked_ticks += 2;
    }
    queued.end_state = Some(SimulationStateSnapshotV1::from_state(&query));
    Ok(RecoveryComparison {
        origin: SimulationStateSnapshotV1::from_state(s),
        requested_command: requested,
        prediction_ticks: ticks,
        commands,
        queued_program: queued,
    })
}
