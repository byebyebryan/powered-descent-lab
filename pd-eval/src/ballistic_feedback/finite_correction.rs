//! Read-only finite acquisition query. Never substitutes a desired live state.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Acquisition {
    pub origin: SimulationStateSnapshotV1,
    pub checked_powered_ticks: u64,
    pub cutoff: Option<SimulationStateSnapshotV1>,
    pub coast_miss_m: Option<f64>,
    pub conflict: Option<PredictedConflict>,
    pub domain_stop: Option<TerrainDomainStop>,
    pub coast_domain_error: Option<TerrainQueryError>,
    pub rejection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coast_settling: Option<phase_transition::Query>,
}

impl Acquisition {
    pub(super) fn accepted(&self) -> bool {
        self.rejection.is_none()
            && self.conflict.is_none()
            && self.domain_stop.is_none()
            && self.coast_domain_error.is_none()
            && self.cutoff.is_some()
            && self.coast_miss_m.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub waypoint_room: Option<HandoffBrakingRoomEstimate>,
    pub desired_arc: Option<BallisticAim>,
    pub correction: Option<Correction>,
    pub ideal_conflict: Option<PredictedConflict>,
    pub ideal_domain_error: Option<TerrainQueryError>,
    pub acquisition: Option<Acquisition>,
    pub rejection: Option<String>,
}

impl Query {
    pub(super) fn retention(
        arc: &BallisticAim,
        correction: &Correction,
        ideal_conflict: Option<PredictedConflict>,
        acquisition: Acquisition,
    ) -> Self {
        Self {
            waypoint_room: None,
            desired_arc: Some(arc.clone()),
            correction: Some(correction.clone()),
            ideal_conflict,
            ideal_domain_error: None,
            rejection: acquisition.rejection.clone(),
            acquisition: Some(acquisition),
        }
    }
}

/// Audit precisely one existing correction, not a new trajectory search. The
/// ordinary short guard is retained at every controller pair, even near cutoff.
#[cfg(test)]
pub(super) fn audit(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    correction: Option<&Correction>,
) -> Result<Acquisition> {
    audit_for(
        request,
        ctx,
        s,
        goal,
        correction,
        WaypointExperiment::FiniteCorrection,
    )
}

pub(super) fn audit_for(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    goal: &Goal,
    correction: Option<&Correction>,
    experiment: WaypointExperiment,
) -> Result<Acquisition> {
    let mut evidence = Acquisition {
        origin: SimulationStateSnapshotV1::from_state(s),
        checked_powered_ticks: 0,
        cutoff: None,
        coast_miss_m: None,
        conflict: None,
        domain_stop: None,
        coast_domain_error: None,
        rejection: None,
        coast_settling: None,
    };
    let mut query = s.clone();
    if let Some(c) = correction {
        ensure!(
            c.burn_end_physics_step > s.physics_step
                && c.burn_end_physics_step <= c.arrival_physics_step
                && c.burn_end_physics_step.is_multiple_of(2)
                && s.physics_step.is_multiple_of(2),
            "invalid finite acquisition clocks"
        );
        while query.physics_step < c.burn_end_physics_step {
            let command = correction_command(ctx, &query, c)?;
            match short_conflict(request, ctx, &query, command, false) {
                Ok(Some(conflict)) => {
                    evidence.conflict = Some(conflict);
                    evidence.rejection = Some("powered_short_guard".into());
                    return Ok(evidence);
                }
                Err(error) => {
                    if let Some(domain) = error.downcast_ref::<TerrainDomainStop>() {
                        let mut diagnostic = domain.clone();
                        diagnostic.actual_state = evidence.origin.clone();
                        evidence.domain_stop = Some(diagnostic);
                        evidence.rejection = Some("powered_prediction_domain".into());
                        return Ok(evidence);
                    }
                    return Err(error);
                }
                Ok(None) => {}
            }
            query.set_command(command);
            for _ in 0..2 {
                query.step_with_contact_report(ctx);
                ensure!(snapshot_finite(&query), "nonfinite finite acquisition");
                evidence.checked_powered_ticks += 1;
                ensure!(
                    !query.is_terminal(),
                    "guarded finite acquisition contacted terrain"
                );
            }
        }
    }
    evidence.cutoff = Some(SimulationStateSnapshotV1::from_state(&query));
    if experiment.phase_queries() {
        let settling = phase_transition::coast_settling(request, ctx, &query)?;
        if experiment.coast_transition() && !settling.accepted() {
            evidence.conflict = settling.conflict.clone();
            evidence.domain_stop = settling.domain_stop.clone();
            evidence.rejection = Some("cutoff_coast_settling_guard".into());
            evidence.coast_settling = Some(settling);
            return Ok(evidence);
        }
        evidence.coast_settling = Some(settling);
    }
    let Some((ticks, miss)) = accepted_coast(ctx, &query, goal) else {
        evidence.rejection = Some("realized_cutoff_approach".into());
        return Ok(evidence);
    };
    evidence.coast_miss_m = Some(miss);
    // Use the realized velocity, not an arc silently redirected at the target.
    let endpoint = aim::project(
        kinematics(&query),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        ticks,
    );
    let arc = aim::target_arc(
        kinematics(&query),
        endpoint.position_m,
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        ticks,
    )
    .context("invalid realized finite coast")?;
    match first_conflict(request, ctx, &query, &arc) {
        Ok(Some(conflict)) => {
            evidence.conflict = Some(conflict);
            evidence.rejection = Some("realized_cutoff_coast_reserve".into());
        }
        Err(error) => {
            if let Some(domain) = error.downcast_ref::<TerrainQueryError>() {
                // The geometric helper returns domain bounds, not a sampled
                // query state. Retain those bounds without inventing a clock.
                evidence.coast_domain_error = Some(domain.clone());
                evidence.rejection = Some("cutoff_coast_domain".into());
            } else {
                return Err(error);
            }
        }
        Ok(None) => match short_conflict(request, ctx, &query, Command::default(), false) {
            Ok(conflict) => {
                if conflict.is_some() {
                    evidence.rejection = Some("cutoff_short_guard".into());
                    evidence.conflict = conflict;
                }
            }
            Err(error) => {
                if let Some(domain) = error.downcast_ref::<TerrainDomainStop>() {
                    let mut diagnostic = domain.clone();
                    diagnostic.actual_state = evidence.origin.clone();
                    evidence.domain_stop = Some(diagnostic);
                    evidence.rejection = Some("cutoff_prediction_domain".into());
                } else {
                    return Err(error);
                }
            }
        },
    }
    Ok(evidence)
}

#[cfg(test)]
pub(super) fn preview(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    coast_steps: u64,
    deadline: u64,
) -> Result<(Query, Option<DestinationPreview>)> {
    preview_for(
        request,
        ctx,
        s,
        coast_steps,
        deadline,
        WaypointExperiment::FiniteCorrection,
    )
}

pub(super) fn preview_for(
    request: &WaypointDirectNominalDirectGenerationRequest,
    ctx: &RunContext,
    s: &SimulationState,
    coast_steps: u64,
    deadline: u64,
    experiment: WaypointExperiment,
) -> Result<(Query, Option<DestinationPreview>)> {
    let entry = aim::project(
        kinematics(s),
        ctx.world.gravity_mps2,
        ctx.sim.physics_dt_s(),
        coast_steps,
    );
    let room = handoff_braking_room(
        (ctx.target_pad.center_x_m - entry.position_m.x).max(0.0),
        entry.velocity_mps.x.max(0.0),
        0.0,
        THRUST_DERATE * ctx.vehicle.max_thrust_n / s.mass_kg(ctx),
        ctx.world.gravity_mps2,
        ctx.vehicle.max_rotation_rate_radps,
        2.0 * ctx.sim.physics_dt_s(),
    );
    let mut evidence = Query {
        waypoint_room: room.clone(),
        desired_arc: None,
        correction: None,
        ideal_conflict: None,
        ideal_domain_error: None,
        acquisition: None,
        rejection: None,
    };
    let Some(room) = room.filter(|room| room.remaining_room_m < 0.0) else {
        evidence.rejection = Some("waypoint_braking_room_sufficient_or_unavailable".into());
        return Ok((evidence, None));
    };
    let goal = target(ctx);
    let fit = if let Some((ticks, _)) = accepted_coast(ctx, s, &goal) {
        aim::target_arc(
            kinematics(s),
            goal.position_m,
            ctx.world.gravity_mps2,
            ctx.sim.physics_dt_s(),
            ticks,
        )
        .map(|arc| (arc, None))
    } else {
        construct_for(ctx, s, &goal, deadline, WaypointExperiment::Combined)
            .map(|(arc, correction)| (arc, Some(correction)))
    };
    let Some((arc, correction)) = fit else {
        evidence.rejection = Some("destination_construction_miss".into());
        return Ok((evidence, None));
    };
    evidence.desired_arc = Some(arc.clone());
    evidence.correction = correction.clone();
    // An optional out-of-domain ideal query declines; it must not terminate the
    // already valid active waypoint. Native powered query domain evidence lives
    // separately in Acquisition.
    match first_conflict(request, ctx, s, &arc) {
        Ok(conflict) => evidence.ideal_conflict = conflict,
        Err(error) if error.downcast_ref::<TerrainQueryError>().is_some() => {
            evidence.ideal_domain_error = error.downcast_ref::<TerrainQueryError>().cloned();
            evidence.rejection = Some("ideal_query_domain".into());
            return Ok((evidence, None));
        }
        Err(error) => return Err(error),
    }
    let acquisition = audit_for(request, ctx, s, &goal, correction.as_ref(), experiment)?;
    let accepted = acquisition.accepted();
    evidence.rejection = acquisition.rejection.clone();
    evidence.acquisition = Some(acquisition);
    Ok((
        evidence,
        accepted.then_some(DestinationPreview {
            goal,
            arc,
            correction,
            waypoint_room: room,
            obstruction: None,
        }),
    ))
}
