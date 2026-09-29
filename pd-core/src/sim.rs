use crate::{
    bounded_run::{
        BOUNDED_RUN_SCHEMA_VERSION, BoundedGuardFailureDispositionV1, BoundedRunArtifactsV1,
        BoundedRunFailureStageV1, BoundedRunFailureV1, BoundedRunGuard, BoundedRunLimitsV1,
        BoundedRunNonFiniteCategoryV1, BoundedRunStopCauseV1, IncomingContactV1,
        SimulationStateSnapshotV1, SimulationStepReportV1,
    },
    eval::{
        ContactClassification, apply_contact_classification, apply_max_time,
        apply_progress_evaluation,
    },
    math::Vec2,
    model::{
        ActionLogEntry, CheckpointRunSummary, Command, EndReason, EvaluationGoal, EventKind,
        EventRecord, LandingRunSummary, MissionOutcome, Observation, PhysicalOutcome,
        RUN_SCHEMA_VERSION, RunArtifacts, RunContext, RunManifest, RunSummary, SampleRecord,
        WaypointSequenceRunSummary,
    },
};

#[derive(Debug)]
pub enum SimulationError {
    InvalidContext(String),
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContext(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SimulationError {}

#[derive(Clone, Debug)]
pub struct SimulationState {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub held_command: Command,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub end_reason: EndReason,
    pub min_touchdown_clearance_m: f64,
    pub min_hull_clearance_m: f64,
    pub max_speed_mps: f64,
    pub max_abs_attitude_rad: f64,
    pub max_abs_angular_rate_radps: f64,
    pub waypoint_sequence_passed: usize,
    pub waypoint_sequence_first_failure_index: Option<usize>,
    pub waypoint_handoff_window_index: Option<usize>,
}

impl SimulationState {
    pub fn new(ctx: &RunContext) -> Result<Self, SimulationError> {
        ctx.sim
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.world
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.vehicle
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.initial_state
            .validate()
            .map_err(SimulationError::InvalidContext)?;

        let mut state = Self {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: ctx.initial_state.position_m,
            velocity_mps: ctx.initial_state.velocity_mps,
            attitude_rad: ctx.initial_state.attitude_rad,
            angular_rate_radps: ctx.initial_state.angular_rate_radps,
            fuel_kg: ctx.vehicle.initial_fuel_kg,
            held_command: Command::idle(),
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
            min_touchdown_clearance_m: f64::INFINITY,
            min_hull_clearance_m: f64::INFINITY,
            max_speed_mps: 0.0,
            max_abs_attitude_rad: 0.0,
            max_abs_angular_rate_radps: 0.0,
            waypoint_sequence_passed: 0,
            waypoint_sequence_first_failure_index: None,
            waypoint_handoff_window_index: None,
        };
        state.update_extrema(ctx);
        Ok(state)
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self.end_reason, EndReason::Running)
    }

    pub fn set_command(&mut self, command: Command) {
        self.held_command = command.clamped();
    }

    pub fn mass_kg(&self, ctx: &RunContext) -> f64 {
        ctx.vehicle.dry_mass_kg + self.fuel_kg.max(0.0)
    }

    pub fn build_observation(&self, ctx: &RunContext) -> Observation {
        let landing_snapshot = self.landing_snapshot(ctx);

        Observation {
            sim_time_s: self.sim_time_s,
            physics_step: self.physics_step,
            position_m: self.position_m,
            velocity_mps: self.velocity_mps,
            attitude_rad: self.attitude_rad,
            angular_rate_radps: self.angular_rate_radps,
            mass_kg: self.mass_kg(ctx),
            fuel_kg: self.fuel_kg,
            gravity_mps2: ctx.world.gravity_mps2,
            target_dx_m: ctx.target_pad.center_x_m - self.position_m.x,
            height_above_target_m: self.position_m.y - ctx.target_pad.surface_y_m,
            target_surface_y_m: ctx.target_pad.surface_y_m,
            target_pad_half_width_m: ctx.target_pad.half_width_m(),
            touchdown_clearance_m: landing_snapshot.min_touchdown_clearance_m,
            min_hull_clearance_m: landing_snapshot.min_hull_clearance_m,
        }
    }

    pub fn step(&mut self, ctx: &RunContext) -> Vec<EventRecord> {
        self.step_with_contact_report_inner(ctx, false).events
    }

    /// Advance one ordinary transition while also retaining any incoming
    /// contact state before mission handling normalizes a stable touchdown.
    pub fn step_with_contact_report(&mut self, ctx: &RunContext) -> SimulationStepReportV1 {
        self.step_with_contact_report_inner(ctx, true)
    }

    fn step_with_contact_report_inner(
        &mut self,
        ctx: &RunContext,
        capture_contact: bool,
    ) -> SimulationStepReportV1 {
        if self.is_terminal() {
            return SimulationStepReportV1::default();
        }

        let contact = self.step_physics_and_classify_contact(ctx);
        let incoming_contact =
            (capture_contact && contact != ContactClassification::None).then(|| {
                IncomingContactV1 {
                    classification: contact.clone(),
                    state: SimulationStateSnapshotV1::from_state(self),
                }
            });
        let contact_events = apply_contact_classification(ctx, self, contact);
        if self.is_terminal() {
            return SimulationStepReportV1 {
                incoming_contact,
                events: contact_events,
            };
        }

        let progress_events = apply_progress_evaluation(ctx, self);
        if self.is_terminal() {
            return SimulationStepReportV1 {
                incoming_contact,
                events: progress_events,
            };
        }

        if self.sim_time_s >= ctx.sim.max_time_s {
            return SimulationStepReportV1 {
                incoming_contact,
                events: apply_max_time(self),
            };
        }

        SimulationStepReportV1 {
            incoming_contact,
            events: contact_events.into_iter().chain(progress_events).collect(),
        }
    }

    /// Advance exactly one discrete plant transition and return the
    /// authoritative post-step contact classification.
    ///
    /// This deliberately stops before mission contact handling, waypoint
    /// progress, and the maximum-time terminal transition.  It is the narrow
    /// neutral seam used by controller-independent proof/replay code.  The
    /// caller must invoke it only while the state is non-terminal, matching
    /// the guard used by [`Self::step`].
    pub fn step_physics_and_classify_contact(&mut self, ctx: &RunContext) -> ContactClassification {
        let dt_s = ctx.sim.physics_dt_s();
        self.apply_attitude_command(ctx, dt_s);
        let throttle_frac = self.consume_fuel(ctx, dt_s);
        self.integrate_translation(ctx, dt_s, throttle_frac);

        self.physics_step += 1;
        self.sim_time_s = self.physics_step as f64 / f64::from(ctx.sim.physics_hz);
        self.update_extrema(ctx);

        self.detect_contact_classification(ctx)
    }

    fn apply_attitude_command(&mut self, ctx: &RunContext, dt_s: f64) {
        let max_delta = ctx.vehicle.max_rotation_rate_radps * dt_s;
        let delta = shortest_angle_delta(self.attitude_rad, self.held_command.target_attitude_rad);
        let applied_delta = delta.clamp(-max_delta, max_delta);
        self.attitude_rad += applied_delta;
        self.angular_rate_radps = applied_delta / dt_s;
    }

    fn consume_fuel(&mut self, ctx: &RunContext, dt_s: f64) -> f64 {
        let throttle_frac = self.applied_throttle_frac(ctx);
        let fuel_used = (ctx.vehicle.max_fuel_burn_kgps * throttle_frac * dt_s).min(self.fuel_kg);
        self.fuel_kg -= fuel_used;
        throttle_frac
    }

    fn applied_throttle_frac(&self, ctx: &RunContext) -> f64 {
        if self.fuel_kg <= 0.0 {
            return 0.0;
        }

        let commanded = self.held_command.throttle_frac.clamp(0.0, 1.0);
        if commanded <= 0.0 {
            return 0.0;
        }

        let min_throttle = ctx.vehicle.min_throttle_frac.clamp(0.0, 1.0);
        min_throttle + commanded * (1.0 - min_throttle)
    }

    fn integrate_translation(&mut self, ctx: &RunContext, dt_s: f64, throttle_frac: f64) {
        let thrust_n = ctx.vehicle.max_thrust_n * throttle_frac;
        let mass_kg = self.mass_kg(ctx).max(1.0);
        let (sin_a, cos_a) = self.attitude_rad.sin_cos();
        let thrust_accel_mps2 =
            Vec2::new((thrust_n / mass_kg) * sin_a, (thrust_n / mass_kg) * cos_a);
        let total_accel_mps2 = Vec2::new(
            thrust_accel_mps2.x,
            thrust_accel_mps2.y - ctx.world.gravity_mps2,
        );

        self.velocity_mps += total_accel_mps2 * dt_s;
        self.position_m += self.velocity_mps * dt_s;
    }

    fn update_extrema(&mut self, ctx: &RunContext) {
        let landing_snapshot = self.landing_snapshot(ctx);
        self.min_touchdown_clearance_m = self
            .min_touchdown_clearance_m
            .min(landing_snapshot.min_touchdown_clearance_m);
        self.min_hull_clearance_m = self
            .min_hull_clearance_m
            .min(landing_snapshot.min_hull_clearance_m);
        self.max_speed_mps = self.max_speed_mps.max(self.velocity_mps.length());
        self.max_abs_attitude_rad = self.max_abs_attitude_rad.max(self.attitude_rad.abs());
        self.max_abs_angular_rate_radps = self
            .max_abs_angular_rate_radps
            .max(self.angular_rate_radps.abs());
    }

    fn detect_contact_classification(&self, ctx: &RunContext) -> ContactClassification {
        let snapshot = self.landing_snapshot(ctx);

        if snapshot.min_touchdown_clearance_m > 0.0 && snapshot.min_hull_clearance_m > 0.0 {
            return ContactClassification::None;
        }

        let hull_radius_m = (ctx.vehicle.geometry.hull_width_m * 0.5)
            .hypot(ctx.vehicle.geometry.hull_height_m * 0.5);
        let contact_closing_speed_mps =
            snapshot.normal_speed_mps + snapshot.angular_rate_radps * hull_radius_m;
        let hull_penetration_tolerance_m =
            0.012_f64.max(contact_closing_speed_mps * ctx.sim.physics_dt_s());
        let stable_touchdown = snapshot.min_touchdown_clearance_m <= 0.05
            && snapshot.max_touchdown_clearance_m <= 0.15
            && snapshot.min_hull_clearance_m >= -hull_penetration_tolerance_m;
        let safe_touchdown = snapshot.normal_speed_mps
            <= ctx.vehicle.safe_touchdown_normal_speed_mps
            && snapshot.tangential_speed_mps <= ctx.vehicle.safe_touchdown_tangential_speed_mps
            && snapshot.attitude_error_rad <= ctx.vehicle.safe_touchdown_attitude_error_rad
            && snapshot.angular_rate_radps <= ctx.vehicle.safe_touchdown_angular_rate_radps;

        if stable_touchdown && safe_touchdown {
            return ContactClassification::StableTouchdown {
                on_target: snapshot.on_target,
            };
        }

        ContactClassification::Crash
    }

    fn landing_snapshot(&self, ctx: &RunContext) -> LandingSnapshot {
        let touchdown_points = self.touchdown_points_world(ctx);
        let touchdown_clearances_m =
            touchdown_points.map(|point| point.y - ctx.world.terrain.sample_height(point.x));
        let min_touchdown_clearance_m = touchdown_clearances_m
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_touchdown_clearance_m = touchdown_clearances_m
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_hull_clearance_m = self.min_hull_clearance_m(ctx);
        let target_surface_normal = ctx
            .world
            .terrain
            .sample_surface_normal(ctx.target_pad.center_x_m);
        let target_surface_tangent = Vec2::new(target_surface_normal.y, -target_surface_normal.x);
        let normal_speed_mps = (-dot(self.velocity_mps, target_surface_normal)).max(0.0);
        let tangential_speed_mps = dot(self.velocity_mps, target_surface_tangent).abs();
        let vehicle_up = Vec2::new(self.attitude_rad.sin(), self.attitude_rad.cos());
        let attitude_error_rad = dot(vehicle_up, target_surface_normal)
            .clamp(-1.0, 1.0)
            .acos();
        let angular_rate_radps = self.angular_rate_radps.abs();
        let touchdown_x_min = touchdown_points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let touchdown_x_max = touchdown_points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let touchdown_center_x_m = (touchdown_x_min + touchdown_x_max) * 0.5;
        let pad_x_min = ctx.target_pad.center_x_m - ctx.target_pad.half_width_m();
        let pad_x_max = ctx.target_pad.center_x_m + ctx.target_pad.half_width_m();
        let on_target = touchdown_x_min >= pad_x_min && touchdown_x_max <= pad_x_max;

        LandingSnapshot {
            min_touchdown_clearance_m,
            max_touchdown_clearance_m,
            min_hull_clearance_m,
            normal_speed_mps,
            tangential_speed_mps,
            attitude_error_rad,
            angular_rate_radps,
            touchdown_center_offset_m: touchdown_center_x_m - ctx.target_pad.center_x_m,
            on_target,
        }
    }

    fn touchdown_points_world(&self, ctx: &RunContext) -> [Vec2; 2] {
        let geometry = &ctx.vehicle.geometry;
        let left_local = Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        );
        let right_local = Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        );

        [
            self.position_m + left_local.rotated(self.attitude_rad),
            self.position_m + right_local.rotated(self.attitude_rad),
        ]
    }

    fn hull_vertices_world(&self, ctx: &RunContext) -> [Vec2; 4] {
        let geometry = &ctx.vehicle.geometry;
        let half_w = geometry.hull_width_m * 0.5;
        let half_h = geometry.hull_height_m * 0.5;
        let local = [
            Vec2::new(-half_w, -half_h),
            Vec2::new(half_w, -half_h),
            Vec2::new(half_w, half_h),
            Vec2::new(-half_w, half_h),
        ];
        local.map(|point| self.position_m + point.rotated(self.attitude_rad))
    }

    fn min_hull_clearance_m(&self, ctx: &RunContext) -> f64 {
        self.hull_vertices_world(ctx)
            .iter()
            .map(|point| point.y - ctx.world.terrain.sample_height(point.x))
            .fold(f64::INFINITY, f64::min)
    }

    fn build_run_summary(&self, ctx: &RunContext) -> RunSummary {
        let landing_snapshot = self.landing_snapshot(ctx);
        let landing = build_landing_run_summary(ctx, &landing_snapshot);
        let checkpoint = build_checkpoint_run_summary(ctx, self);
        let waypoint_sequence = build_waypoint_sequence_run_summary(ctx, self);
        let envelope_margin_ratio = checkpoint
            .as_ref()
            .map(|summary| summary.envelope_margin_ratio)
            .or_else(|| {
                landing
                    .as_ref()
                    .map(|summary| summary.envelope_margin_ratio)
            });

        RunSummary {
            fuel_remaining_kg: self.fuel_kg.max(0.0),
            fuel_used_kg: (ctx.vehicle.initial_fuel_kg - self.fuel_kg).max(0.0),
            min_touchdown_clearance_m: self.min_touchdown_clearance_m,
            min_hull_clearance_m: self.min_hull_clearance_m,
            max_speed_mps: self.max_speed_mps,
            max_abs_attitude_rad: self.max_abs_attitude_rad,
            max_abs_angular_rate_radps: self.max_abs_angular_rate_radps,
            envelope_margin_ratio,
            landing,
            checkpoint,
            waypoint_sequence,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LandingSnapshot {
    min_touchdown_clearance_m: f64,
    max_touchdown_clearance_m: f64,
    min_hull_clearance_m: f64,
    normal_speed_mps: f64,
    tangential_speed_mps: f64,
    attitude_error_rad: f64,
    angular_rate_radps: f64,
    touchdown_center_offset_m: f64,
    on_target: bool,
}

pub fn run_simulation<F>(
    ctx: &RunContext,
    controller_id: &str,
    mut controller: F,
) -> Result<RunArtifacts, SimulationError>
where
    F: FnMut(&RunContext, &Observation) -> Command,
{
    let mut state = SimulationState::new(ctx)?;
    let mut actions = Vec::new();
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let control_interval_steps = ctx.sim.control_interval_steps();
    let sample_interval_steps = ctx.sim.sample_interval_steps();
    let mut controller_update_index = 0_u64;

    maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);
    issue_controller_update(
        &mut state,
        &mut controller,
        &mut controller_update_index,
        &mut actions,
        &mut events,
        ctx,
    );

    while !state.is_terminal() {
        events.extend(state.step(ctx));
        maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);

        if state.is_terminal() {
            break;
        }

        if state.physics_step % control_interval_steps == 0 {
            issue_controller_update(
                &mut state,
                &mut controller,
                &mut controller_update_index,
                &mut actions,
                &mut events,
                ctx,
            );
        }
    }

    Ok(RunArtifacts {
        manifest: build_manifest(ctx, controller_id, &state, controller_update_index),
        actions,
        events,
        samples,
    })
}

pub fn replay_simulation(
    ctx: &RunContext,
    controller_id: &str,
    actions: &[ActionLogEntry],
) -> Result<RunArtifacts, SimulationError> {
    if actions.is_empty() {
        return Err(SimulationError::InvalidContext(
            "action log must contain at least one controller update".to_owned(),
        ));
    }

    let mut state = SimulationState::new(ctx)?;
    let mut replay_actions = Vec::with_capacity(actions.len());
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let control_interval_steps = ctx.sim.control_interval_steps();
    let sample_interval_steps = ctx.sim.sample_interval_steps();
    let mut next_action_index = 0_usize;

    maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);
    consume_replay_action(
        ctx,
        &mut state,
        actions,
        &mut next_action_index,
        &mut replay_actions,
        &mut events,
    )?;

    while !state.is_terminal() {
        events.extend(state.step(ctx));
        maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);

        if state.is_terminal() {
            break;
        }

        if state.physics_step % control_interval_steps == 0 {
            consume_replay_action(
                ctx,
                &mut state,
                actions,
                &mut next_action_index,
                &mut replay_actions,
                &mut events,
            )?;
        }
    }

    if next_action_index != actions.len() {
        return Err(SimulationError::InvalidContext(format!(
            "action log contains {} extra controller updates after termination",
            actions.len() - next_action_index
        )));
    }

    Ok(RunArtifacts {
        manifest: build_manifest(ctx, controller_id, &state, replay_actions.len() as u64),
        actions: replay_actions,
        events,
        samples,
    })
}

/// Execute an ordinary simulation with finite saved-command and hard-end
/// boundaries. Unlike `run_simulation`, command selection can fail without
/// discarding the valid artifact prefix.
pub fn run_simulation_bounded<F>(
    ctx: &RunContext,
    controller_id: &str,
    limits: BoundedRunLimitsV1,
    mut controller: F,
    guard: &mut dyn BoundedRunGuard,
) -> Result<BoundedRunArtifactsV1, SimulationError>
where
    F: FnMut(&RunContext, &Observation) -> Result<Command, String>,
{
    let mut state = SimulationState::new(ctx)?;
    validate_bounded_limits(ctx, limits)?;
    let control_interval_steps = ctx.sim.control_interval_steps();
    let sample_interval_steps = ctx.sim.sample_interval_steps();
    let mut actions = Vec::new();
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let mut controller_update_index = 0_u64;
    let mut incoming_contact = None;
    if let Some(failure) = first_nonfinite_artifact_projection_failure(
        ctx,
        &state,
        BoundedRunFailureStageV1::InitialGuard,
        state.physics_step,
    ) {
        return Err(SimulationError::InvalidContext(format!(
            "bounded run has no finite initial state: {}",
            failure.reason
        )));
    }

    maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);

    let failure = if let Err(guard_failure) = guard.initial(ctx, &state) {
        Some(wrap_guard_failure(
            BoundedRunFailureStageV1::InitialGuard,
            &state,
            guard_failure,
        ))
    } else {
        None
    };
    if let Some(failure) = failure {
        let stop = stop_for_failure(&failure);
        return Ok(build_bounded_run_artifacts(
            ctx,
            controller_id,
            limits,
            stop,
            Some(failure),
            incoming_contact,
            state,
            controller_update_index,
            actions,
            events,
            samples,
        ));
    }

    if let Some(stop) = bounded_stop_at_state(&state, limits) {
        return Ok(build_bounded_run_artifacts(
            ctx,
            controller_id,
            limits,
            stop,
            None,
            incoming_contact,
            state,
            controller_update_index,
            actions,
            events,
            samples,
        ));
    }

    if let Err(command_failure) = issue_bounded_controller_update(
        ctx,
        &mut state,
        &mut controller,
        &mut controller_update_index,
        &mut actions,
        &mut events,
    ) {
        let stop = stop_for_failure(&command_failure);
        return Ok(build_bounded_run_artifacts(
            ctx,
            controller_id,
            limits,
            stop,
            Some(command_failure),
            incoming_contact,
            state,
            controller_update_index,
            actions,
            events,
            samples,
        ));
    }

    let mut failure = None;
    let stop = loop {
        if let Err(guard_failure) = guard.before_transition(ctx, &state, state.held_command) {
            let pre_failure = wrap_guard_failure(
                BoundedRunFailureStageV1::PreTransitionGuard,
                &state,
                guard_failure,
            );
            let stop = stop_for_failure(&pre_failure);
            failure = Some(pre_failure);
            break stop;
        }

        let last_valid_state = state.clone();
        let transition = state.step_with_contact_report(ctx);
        if let Some(numeric_failure) = first_nonfinite_transition_failure(ctx, &state, &transition)
        {
            state = last_valid_state;
            let stop = stop_for_failure(&numeric_failure);
            failure = Some(numeric_failure);
            break stop;
        }

        events.extend(transition.events);
        maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);
        let transition_contact = transition.incoming_contact;
        if let Some(contact) = &transition_contact {
            incoming_contact = Some(contact.clone());
        }

        if let Err(guard_failure) = guard.after_transition(ctx, &state, transition_contact.as_ref())
        {
            let post_failure = wrap_guard_failure(
                BoundedRunFailureStageV1::PostTransitionGuard,
                &state,
                guard_failure,
            );
            let stop = stop_for_failure(&post_failure);
            failure = Some(post_failure);
            break stop;
        }

        if state.is_terminal() {
            break if matches!(state.end_reason, EndReason::MaxTimeReached) {
                BoundedRunStopCauseV1::ScenarioHorizonReached
            } else {
                BoundedRunStopCauseV1::MissionTerminal
            };
        }

        if let Some(stop) = bounded_stop_at_state(&state, limits) {
            break stop;
        }

        if state.physics_step.is_multiple_of(control_interval_steps)
            && let Err(command_failure) = issue_bounded_controller_update(
                ctx,
                &mut state,
                &mut controller,
                &mut controller_update_index,
                &mut actions,
                &mut events,
            )
        {
            let stop = stop_for_failure(&command_failure);
            failure = Some(command_failure);
            break stop;
        }
    };

    Ok(build_bounded_run_artifacts(
        ctx,
        controller_id,
        limits,
        stop,
        failure,
        incoming_contact,
        state,
        controller_update_index,
        actions,
        events,
        samples,
    ))
}

/// Replay a bounded artifact action prefix under caller-supplied limits and a
/// fresh guard. The returned envelope retains valid actions if replay input
/// becomes invalid at a later callback or contains unused records.
pub fn replay_simulation_bounded(
    ctx: &RunContext,
    controller_id: &str,
    actions: &[ActionLogEntry],
    limits: BoundedRunLimitsV1,
    guard: &mut dyn BoundedRunGuard,
) -> Result<BoundedRunArtifactsV1, SimulationError> {
    let mut next_action_index = 0_usize;
    let mut nonfinite_action_failure = None;
    let mut result = run_simulation_bounded(
        ctx,
        controller_id,
        limits,
        |_, observation| {
            let index = next_action_index;
            let Some(action) = actions.get(index) else {
                return Err(format!(
                    "action log ended before controller update {index} at physics step {}",
                    observation.physics_step
                ));
            };
            if let Some(failure) = nonfinite_value_failure(
                "action.sim_time_s",
                action.sim_time_s,
                BoundedRunFailureStageV1::ReplayInput,
                observation.physics_step,
            ) {
                let reason = failure.reason.clone();
                nonfinite_action_failure = Some(failure);
                return Err(reason);
            }
            if let Some(failure) = nonfinite_command_failure(
                action.command,
                BoundedRunFailureStageV1::ReplayInput,
                observation.physics_step,
            ) {
                let reason = failure.reason.clone();
                nonfinite_action_failure = Some(failure);
                return Err(reason);
            }
            validate_bounded_action(action, index, observation, ctx)?;
            next_action_index += 1;
            Ok(action.command)
        },
        guard,
    )?;

    if let Some(failure) = result.failure.as_mut()
        && failure.stage == BoundedRunFailureStageV1::CommandSelection
    {
        failure.stage = BoundedRunFailureStageV1::ReplayInput;
    }
    if let Some(failure) = nonfinite_action_failure {
        result.stop = BoundedRunStopCauseV1::ExecutionInvalid;
        result.failure = Some(failure);
    }

    if next_action_index != actions.len() {
        let unused = actions.len() - next_action_index;
        let reason = match result.failure.as_ref() {
            Some(prior)
                if prior.disposition == BoundedGuardFailureDispositionV1::SafetyRejected =>
            {
                format!(
                    "action log contains {unused} unused controller updates after a safety rejection at physics step {}: {}",
                    prior.boundary_physics_step, prior.reason
                )
            }
            Some(_) => return Ok(result),
            None => format!(
                "action log contains {unused} unused controller updates after bounded termination"
            ),
        };
        let failure = BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::ReplayInput,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            result.final_state.physics_step,
            reason,
        );
        result.stop = BoundedRunStopCauseV1::ExecutionInvalid;
        result.failure = Some(failure);
    }

    Ok(result)
}

fn validate_bounded_limits(
    ctx: &RunContext,
    limits: BoundedRunLimitsV1,
) -> Result<(), SimulationError> {
    let interval = ctx.sim.control_interval_steps();
    if !limits
        .command_coverage_end_physics_step
        .is_multiple_of(interval)
    {
        return Err(SimulationError::InvalidContext(
            "bounded command coverage end must be on the global control clock".to_owned(),
        ));
    }
    let horizon_product = ctx.sim.max_time_s * f64::from(ctx.sim.physics_hz);
    if !horizon_product.is_finite() || horizon_product.ceil() > u64::MAX as f64 {
        return Err(SimulationError::InvalidContext(
            "simulation horizon is not representable in physics steps".to_owned(),
        ));
    }
    let horizon_steps = horizon_product.ceil() as u64;
    if limits.hard_end_physics_step > horizon_steps {
        return Err(SimulationError::InvalidContext(
            "bounded hard end exceeds the simulation horizon".to_owned(),
        ));
    }
    Ok(())
}

fn bounded_stop_at_state(
    state: &SimulationState,
    limits: BoundedRunLimitsV1,
) -> Option<BoundedRunStopCauseV1> {
    if state.physics_step >= limits.hard_end_physics_step {
        Some(BoundedRunStopCauseV1::HardDeadlineReached)
    } else if state.physics_step >= limits.command_coverage_end_physics_step {
        Some(BoundedRunStopCauseV1::CoverageExhausted)
    } else {
        None
    }
}

fn wrap_guard_failure(
    stage: BoundedRunFailureStageV1,
    state: &SimulationState,
    failure: crate::bounded_run::BoundedGuardFailureV1,
) -> BoundedRunFailureV1 {
    BoundedRunFailureV1::new(
        stage,
        failure.disposition,
        state.physics_step,
        failure.reason,
    )
}

fn stop_for_failure(failure: &BoundedRunFailureV1) -> BoundedRunStopCauseV1 {
    match failure.disposition {
        BoundedGuardFailureDispositionV1::SafetyRejected => BoundedRunStopCauseV1::SafetyRejected,
        BoundedGuardFailureDispositionV1::ExecutionInvalid => {
            BoundedRunStopCauseV1::ExecutionInvalid
        }
    }
}

fn issue_bounded_controller_update<F>(
    ctx: &RunContext,
    state: &mut SimulationState,
    controller: &mut F,
    controller_update_index: &mut u64,
    actions: &mut Vec<ActionLogEntry>,
    events: &mut Vec<EventRecord>,
) -> Result<(), BoundedRunFailureV1>
where
    F: FnMut(&RunContext, &Observation) -> Result<Command, String>,
{
    let observation = state.build_observation(ctx);
    let command = controller(ctx, &observation).map_err(|reason| {
        BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::CommandSelection,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            state.physics_step,
            reason,
        )
    })?;
    if let Some(failure) = nonfinite_command_failure(
        command,
        BoundedRunFailureStageV1::CommandSelection,
        state.physics_step,
    ) {
        return Err(failure);
    }
    if let Some(reason) = invalid_bounded_command(command) {
        return Err(BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::CommandSelection,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            state.physics_step,
            reason,
        ));
    }
    let next_index = controller_update_index.checked_add(1).ok_or_else(|| {
        BoundedRunFailureV1::new(
            BoundedRunFailureStageV1::CommandSelection,
            BoundedGuardFailureDispositionV1::ExecutionInvalid,
            state.physics_step,
            "controller update count overflow",
        )
    })?;
    state.set_command(command);
    actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: *controller_update_index,
        command: state.held_command,
    });
    events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".to_owned(),
    });
    *controller_update_index = next_index;
    Ok(())
}

fn validate_bounded_action(
    action: &ActionLogEntry,
    index: usize,
    observation: &Observation,
    ctx: &RunContext,
) -> Result<(), String> {
    if action.physics_step != observation.physics_step {
        return Err(format!(
            "action {index} expected physics_step {}, got {}",
            observation.physics_step, action.physics_step
        ));
    }
    if action.controller_update_index != index as u64 {
        return Err(format!(
            "action {index} expected controller_update_index {index}, got {}",
            action.controller_update_index
        ));
    }
    if action.sim_time_s.to_bits() != observation.sim_time_s.to_bits() {
        return Err(format!(
            "action {index} expected sim_time_s {:.12}, got {:.12}",
            observation.sim_time_s, action.sim_time_s
        ));
    }
    if action.physics_step != 0
        && !action
            .physics_step
            .is_multiple_of(ctx.sim.control_interval_steps())
    {
        return Err(format!("action {index} is not on the global control clock"));
    }
    invalid_bounded_command(action.command).map_or(Ok(()), Err)
}

fn invalid_bounded_command(command: Command) -> Option<String> {
    if !command.throttle_frac.is_finite() || !command.target_attitude_rad.is_finite() {
        return Some("bounded command values must be finite".to_owned());
    }
    if !(0.0..=1.0).contains(&command.throttle_frac) {
        return Some("bounded command throttle_frac must be within [0, 1]".to_owned());
    }
    if !(-std::f64::consts::PI..=std::f64::consts::PI).contains(&command.target_attitude_rad) {
        return Some("bounded command target attitude must be canonical".to_owned());
    }
    None
}

fn nonfinite_command_failure(
    command: Command,
    stage: BoundedRunFailureStageV1,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    [
        ("command.throttle_frac", command.throttle_frac),
        ("command.target_attitude_rad", command.target_attitude_rad),
    ]
    .into_iter()
    .find_map(|(field, value)| nonfinite_value_failure(field, value, stage, boundary_physics_step))
}

fn nonfinite_value_failure(
    field: impl Into<String>,
    value: f64,
    stage: BoundedRunFailureStageV1,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    let category = if value.is_nan() {
        BoundedRunNonFiniteCategoryV1::Nan
    } else if value == f64::INFINITY {
        BoundedRunNonFiniteCategoryV1::PositiveInfinity
    } else if value == f64::NEG_INFINITY {
        BoundedRunNonFiniteCategoryV1::NegativeInfinity
    } else {
        return None;
    };
    Some(BoundedRunFailureV1::non_finite(
        stage,
        field,
        category,
        boundary_physics_step,
    ))
}

fn first_nonfinite_transition_failure(
    ctx: &RunContext,
    state: &SimulationState,
    transition: &SimulationStepReportV1,
) -> Option<BoundedRunFailureV1> {
    if let Some(failure) = first_nonfinite_artifact_projection_failure(
        ctx,
        state,
        BoundedRunFailureStageV1::PhysicsTransition,
        state.physics_step,
    ) {
        return Some(failure);
    }
    transition.incoming_contact.as_ref().and_then(|contact| {
        first_nonfinite_artifact_projection_failure(
            ctx,
            &contact.state.to_simulation_state(),
            BoundedRunFailureStageV1::PhysicsTransition,
            state.physics_step,
        )
    })
}

fn first_nonfinite_artifact_projection_failure(
    ctx: &RunContext,
    state: &SimulationState,
    stage: BoundedRunFailureStageV1,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    if let Some(failure) = first_nonfinite_state_failure(state, stage, boundary_physics_step) {
        return Some(failure);
    }

    let observation = state.build_observation(ctx);
    let observation_values = [
        ("observation.sim_time_s", observation.sim_time_s),
        ("observation.position_m.x", observation.position_m.x),
        ("observation.position_m.y", observation.position_m.y),
        ("observation.velocity_mps.x", observation.velocity_mps.x),
        ("observation.velocity_mps.y", observation.velocity_mps.y),
        ("observation.attitude_rad", observation.attitude_rad),
        (
            "observation.angular_rate_radps",
            observation.angular_rate_radps,
        ),
        ("observation.mass_kg", observation.mass_kg),
        ("observation.fuel_kg", observation.fuel_kg),
        ("observation.gravity_mps2", observation.gravity_mps2),
        ("observation.target_dx_m", observation.target_dx_m),
        (
            "observation.height_above_target_m",
            observation.height_above_target_m,
        ),
        (
            "observation.target_surface_y_m",
            observation.target_surface_y_m,
        ),
        (
            "observation.target_pad_half_width_m",
            observation.target_pad_half_width_m,
        ),
        (
            "observation.touchdown_clearance_m",
            observation.touchdown_clearance_m,
        ),
        (
            "observation.min_hull_clearance_m",
            observation.min_hull_clearance_m,
        ),
    ];
    for (field, value) in observation_values {
        if let Some(failure) = nonfinite_value_failure(field, value, stage, boundary_physics_step) {
            return Some(failure);
        }
    }

    let summary = state.build_run_summary(ctx);
    let mut summary_values = vec![
        ("summary.fuel_remaining_kg", summary.fuel_remaining_kg),
        ("summary.fuel_used_kg", summary.fuel_used_kg),
        (
            "summary.min_touchdown_clearance_m",
            summary.min_touchdown_clearance_m,
        ),
        ("summary.min_hull_clearance_m", summary.min_hull_clearance_m),
        ("summary.max_speed_mps", summary.max_speed_mps),
        ("summary.max_abs_attitude_rad", summary.max_abs_attitude_rad),
        (
            "summary.max_abs_angular_rate_radps",
            summary.max_abs_angular_rate_radps,
        ),
    ];
    if let Some(value) = summary.envelope_margin_ratio {
        summary_values.push(("summary.envelope_margin_ratio", value));
    }
    if let Some(landing) = &summary.landing {
        summary_values.extend([
            (
                "summary.landing.touchdown_center_offset_m",
                landing.touchdown_center_offset_m,
            ),
            ("summary.landing.pad_margin_m", landing.pad_margin_m),
            ("summary.landing.normal_speed_mps", landing.normal_speed_mps),
            (
                "summary.landing.tangential_speed_mps",
                landing.tangential_speed_mps,
            ),
            (
                "summary.landing.attitude_error_rad",
                landing.attitude_error_rad,
            ),
            (
                "summary.landing.angular_rate_radps",
                landing.angular_rate_radps,
            ),
            (
                "summary.landing.normal_speed_margin_mps",
                landing.normal_speed_margin_mps,
            ),
            (
                "summary.landing.tangential_speed_margin_mps",
                landing.tangential_speed_margin_mps,
            ),
            (
                "summary.landing.attitude_margin_rad",
                landing.attitude_margin_rad,
            ),
            (
                "summary.landing.angular_rate_margin_radps",
                landing.angular_rate_margin_radps,
            ),
            (
                "summary.landing.envelope_margin_ratio",
                landing.envelope_margin_ratio,
            ),
        ]);
    }
    if let Some(checkpoint) = &summary.checkpoint {
        summary_values.extend([
            (
                "summary.checkpoint.position_error_m",
                checkpoint.position_error_m,
            ),
            (
                "summary.checkpoint.velocity_error_mps",
                checkpoint.velocity_error_mps,
            ),
            (
                "summary.checkpoint.attitude_error_rad",
                checkpoint.attitude_error_rad,
            ),
            (
                "summary.checkpoint.position_margin_m",
                checkpoint.position_margin_m,
            ),
            (
                "summary.checkpoint.velocity_margin_mps",
                checkpoint.velocity_margin_mps,
            ),
            (
                "summary.checkpoint.attitude_margin_rad",
                checkpoint.attitude_margin_rad,
            ),
            (
                "summary.checkpoint.envelope_margin_ratio",
                checkpoint.envelope_margin_ratio,
            ),
        ]);
    }
    summary_values.into_iter().find_map(|(field, value)| {
        nonfinite_value_failure(field, value, stage, boundary_physics_step)
    })
}

fn first_nonfinite_state_failure(
    state: &SimulationState,
    stage: BoundedRunFailureStageV1,
    boundary_physics_step: u64,
) -> Option<BoundedRunFailureV1> {
    let values = [
        ("sim_time_s", state.sim_time_s),
        ("position_m.x", state.position_m.x),
        ("position_m.y", state.position_m.y),
        ("velocity_mps.x", state.velocity_mps.x),
        ("velocity_mps.y", state.velocity_mps.y),
        ("attitude_rad", state.attitude_rad),
        ("angular_rate_radps", state.angular_rate_radps),
        ("fuel_kg", state.fuel_kg),
        (
            "held_command.throttle_frac",
            state.held_command.throttle_frac,
        ),
        (
            "held_command.target_attitude_rad",
            state.held_command.target_attitude_rad,
        ),
        ("min_touchdown_clearance_m", state.min_touchdown_clearance_m),
        ("min_hull_clearance_m", state.min_hull_clearance_m),
        ("max_speed_mps", state.max_speed_mps),
        ("max_abs_attitude_rad", state.max_abs_attitude_rad),
        (
            "max_abs_angular_rate_radps",
            state.max_abs_angular_rate_radps,
        ),
    ];
    values.into_iter().find_map(|(field, value)| {
        let category = if value.is_nan() {
            Some(BoundedRunNonFiniteCategoryV1::Nan)
        } else if value == f64::INFINITY {
            Some(BoundedRunNonFiniteCategoryV1::PositiveInfinity)
        } else if value == f64::NEG_INFINITY {
            Some(BoundedRunNonFiniteCategoryV1::NegativeInfinity)
        } else {
            None
        }?;
        Some(BoundedRunFailureV1::non_finite(
            stage,
            field,
            category,
            boundary_physics_step,
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn build_bounded_run_artifacts(
    ctx: &RunContext,
    controller_id: &str,
    limits: BoundedRunLimitsV1,
    stop: BoundedRunStopCauseV1,
    failure: Option<BoundedRunFailureV1>,
    incoming_contact: Option<IncomingContactV1>,
    state: SimulationState,
    controller_updates: u64,
    actions: Vec<ActionLogEntry>,
    events: Vec<EventRecord>,
    samples: Vec<SampleRecord>,
) -> BoundedRunArtifactsV1 {
    let coverage_reached = state.physics_step >= limits.command_coverage_end_physics_step;
    let hard_end_reached = state.physics_step >= limits.hard_end_physics_step;
    let final_state = SimulationStateSnapshotV1::from_state(&state);
    let run = RunArtifacts {
        manifest: build_manifest(ctx, controller_id, &state, controller_updates),
        actions,
        events,
        samples,
    };
    BoundedRunArtifactsV1 {
        schema_version: BOUNDED_RUN_SCHEMA_VERSION,
        limits,
        stop,
        coverage_reached,
        hard_end_reached,
        final_state,
        incoming_contact,
        failure,
        run,
    }
}

fn issue_controller_update<F>(
    state: &mut SimulationState,
    controller: &mut F,
    controller_update_index: &mut u64,
    actions: &mut Vec<ActionLogEntry>,
    events: &mut Vec<EventRecord>,
    ctx: &RunContext,
) where
    F: FnMut(&RunContext, &Observation) -> Command,
{
    let observation = state.build_observation(ctx);
    let command = controller(ctx, &observation).clamped();
    state.set_command(command);

    actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: *controller_update_index,
        command: state.held_command,
    });
    events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".to_owned(),
    });
    *controller_update_index += 1;
}

fn consume_replay_action(
    ctx: &RunContext,
    state: &mut SimulationState,
    actions: &[ActionLogEntry],
    next_action_index: &mut usize,
    replay_actions: &mut Vec<ActionLogEntry>,
    events: &mut Vec<EventRecord>,
) -> Result<(), SimulationError> {
    let Some(action) = actions.get(*next_action_index) else {
        return Err(SimulationError::InvalidContext(format!(
            "action log ended before controller update {} at physics step {}",
            *next_action_index, state.physics_step
        )));
    };

    if action.physics_step != state.physics_step {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected physics_step {}, got {}",
            *next_action_index, state.physics_step, action.physics_step
        )));
    }
    if action.controller_update_index != *next_action_index as u64 {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected controller_update_index {}, got {}",
            *next_action_index, *next_action_index, action.controller_update_index
        )));
    }
    if (action.sim_time_s - state.sim_time_s).abs() > 1e-9 {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected sim_time_s {:.12}, got {:.12}",
            *next_action_index, state.sim_time_s, action.sim_time_s
        )));
    }
    if state.physics_step != 0
        && !state
            .physics_step
            .is_multiple_of(ctx.sim.control_interval_steps())
    {
        return Err(SimulationError::InvalidContext(format!(
            "action {} occurs on invalid control step {}",
            *next_action_index, state.physics_step
        )));
    }

    state.set_command(action.command);
    replay_actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: action.controller_update_index,
        command: state.held_command,
    });
    events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".to_owned(),
    });
    *next_action_index += 1;
    Ok(())
}

fn build_manifest(
    ctx: &RunContext,
    controller_id: &str,
    state: &SimulationState,
    controller_updates: u64,
) -> RunManifest {
    RunManifest {
        schema_version: RUN_SCHEMA_VERSION,
        scenario_id: ctx.scenario_id.clone(),
        scenario_name: ctx.scenario_name.clone(),
        scenario_seed: ctx.scenario_seed,
        scenario_tags: ctx.scenario_tags.clone(),
        controller_id: controller_id.to_owned(),
        physics_hz: ctx.sim.physics_hz,
        controller_hz: ctx.sim.controller_hz,
        sim_time_s: state.sim_time_s,
        physics_steps: state.physics_step,
        controller_updates,
        physical_outcome: state.physical_outcome.clone(),
        mission_outcome: state.mission_outcome.clone(),
        end_reason: state.end_reason.clone(),
        summary: state.build_run_summary(ctx),
    }
}

fn build_landing_run_summary(
    ctx: &RunContext,
    snapshot: &LandingSnapshot,
) -> Option<LandingRunSummary> {
    let pad_margin_m = ctx.target_pad.half_width_m() - snapshot.touchdown_center_offset_m.abs();
    let normal_speed_margin_mps =
        ctx.vehicle.safe_touchdown_normal_speed_mps - snapshot.normal_speed_mps;
    let tangential_speed_margin_mps =
        ctx.vehicle.safe_touchdown_tangential_speed_mps - snapshot.tangential_speed_mps;
    let attitude_margin_rad =
        ctx.vehicle.safe_touchdown_attitude_error_rad - snapshot.attitude_error_rad;
    let angular_rate_margin_radps =
        ctx.vehicle.safe_touchdown_angular_rate_radps - snapshot.angular_rate_radps;
    let envelope_margin_ratio = [
        normal_speed_margin_mps
            / ctx
                .vehicle
                .safe_touchdown_normal_speed_mps
                .max(f64::EPSILON),
        tangential_speed_margin_mps
            / ctx
                .vehicle
                .safe_touchdown_tangential_speed_mps
                .max(f64::EPSILON),
        attitude_margin_rad
            / ctx
                .vehicle
                .safe_touchdown_attitude_error_rad
                .max(f64::EPSILON),
        angular_rate_margin_radps
            / ctx
                .vehicle
                .safe_touchdown_angular_rate_radps
                .max(f64::EPSILON),
        pad_margin_m / ctx.target_pad.half_width_m().max(f64::EPSILON),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min);

    Some(LandingRunSummary {
        touchdown_center_offset_m: snapshot.touchdown_center_offset_m,
        pad_margin_m,
        normal_speed_mps: snapshot.normal_speed_mps,
        tangential_speed_mps: snapshot.tangential_speed_mps,
        attitude_error_rad: snapshot.attitude_error_rad,
        angular_rate_radps: snapshot.angular_rate_radps,
        normal_speed_margin_mps,
        tangential_speed_margin_mps,
        attitude_margin_rad,
        angular_rate_margin_radps,
        envelope_margin_ratio,
        on_target: snapshot.on_target,
    })
}

fn build_checkpoint_run_summary(
    ctx: &RunContext,
    state: &SimulationState,
) -> Option<CheckpointRunSummary> {
    let EvaluationGoal::TimedCheckpoint {
        desired_position_offset_m,
        max_position_error_m,
        desired_velocity_mps,
        max_velocity_error_mps,
        max_attitude_error_rad,
        ..
    } = &ctx.mission.goal
    else {
        return None;
    };

    let actual_position_offset_m = Vec2::new(
        state.position_m.x - ctx.target_pad.center_x_m,
        state.position_m.y - ctx.target_pad.surface_y_m,
    );
    let position_error_m = (actual_position_offset_m - *desired_position_offset_m).length();
    let velocity_error_mps = (state.velocity_mps - *desired_velocity_mps).length();
    let attitude_error_rad = state.attitude_rad.abs();
    let position_margin_m = *max_position_error_m - position_error_m;
    let velocity_margin_mps = *max_velocity_error_mps - velocity_error_mps;
    let attitude_margin_rad = *max_attitude_error_rad - attitude_error_rad;
    let envelope_margin_ratio = [
        position_margin_m / max_position_error_m.max(f64::EPSILON),
        velocity_margin_mps / max_velocity_error_mps.max(f64::EPSILON),
        attitude_margin_rad / max_attitude_error_rad.max(f64::EPSILON),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min);

    Some(CheckpointRunSummary {
        position_error_m,
        velocity_error_mps,
        attitude_error_rad,
        position_margin_m,
        velocity_margin_mps,
        attitude_margin_rad,
        envelope_margin_ratio,
    })
}

fn build_waypoint_sequence_run_summary(
    ctx: &RunContext,
    state: &SimulationState,
) -> Option<WaypointSequenceRunSummary> {
    if !matches!(ctx.mission.goal, EvaluationGoal::WaypointSequence { .. }) {
        return None;
    }

    Some(WaypointSequenceRunSummary {
        passed_handoffs: state.waypoint_sequence_passed,
        total_handoffs: ctx
            .mission
            .transfer_route
            .as_ref()
            .map_or(0, |route| route.waypoints.len()),
        first_failed_index: state.waypoint_sequence_first_failure_index,
    })
}

fn maybe_push_sample(
    samples: &mut Vec<SampleRecord>,
    state: &SimulationState,
    ctx: &RunContext,
    sample_interval_steps: Option<u64>,
) {
    let Some(sample_interval_steps) = sample_interval_steps else {
        return;
    };
    if !state.physics_step.is_multiple_of(sample_interval_steps) && !state.is_terminal() {
        return;
    }
    if samples
        .last()
        .is_some_and(|sample| sample.physics_step == state.physics_step)
    {
        return;
    }

    samples.push(SampleRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        observation: state.build_observation(ctx),
        held_command: state.held_command,
    });
}

fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let two_pi = std::f64::consts::TAU;
    let mut delta = (target_rad - current_rad) % two_pi;
    if delta > std::f64::consts::PI {
        delta -= two_pi;
    } else if delta < -std::f64::consts::PI {
        delta += two_pi;
    }
    delta
}

fn dot(lhs: Vec2, rhs: Vec2) -> f64 {
    (lhs.x * rhs.x) + (lhs.y * rhs.y)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::AllowAllBoundedRunGuard;
    use crate::model::{
        EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, VehicleGeometry,
        VehicleInitialState, VehicleSpec, WorldSpec,
    };
    use crate::terrain::TerrainDefinition;

    fn smoke_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "smoke".to_owned(),
            name: "Smoke".to_owned(),
            description: "smoke test".to_owned(),
            seed: 1,
            tags: vec!["test".to_owned(), "smoke".to_owned()],
            metadata: BTreeMap::from([("suite".to_owned(), "unit".to_owned())]),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-50.0, 0.0), Vec2::new(50.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 30.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 14_000.0,
                max_fuel_burn_kgps: 10.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 15.0),
                velocity_mps: Vec2::new(0.0, -5.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                transfer_route: None,
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "pad_a".to_owned(),
                },
            },
        }
    }

    fn contact_state_with_hull_penetration(
        hull_penetration_m: f64,
        normal_speed_mps: f64,
        tangential_speed_mps: f64,
    ) -> (RunContext, SimulationState) {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        let half_h = ctx.vehicle.geometry.hull_height_m * 0.5;
        state.position_m = Vec2::new(0.0, half_h - hull_penetration_m);
        state.velocity_mps = Vec2::new(tangential_speed_mps, -normal_speed_mps);
        state.attitude_rad = 0.0;
        state.angular_rate_radps = 0.0;
        state.update_extrema(&ctx);
        (ctx, state)
    }

    #[test]
    fn run_simulation_emits_authoritative_logs() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let artifacts = run_simulation(&ctx, "idle", |_, _| Command::idle()).unwrap();

        assert!(!artifacts.actions.is_empty());
        assert!(!artifacts.events.is_empty());
        assert!(matches!(
            artifacts.manifest.end_reason,
            EndReason::Crash | EndReason::MaxTimeReached
        ));
    }

    #[test]
    fn replay_simulation_reproduces_manifest_and_events() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let original = run_simulation(&ctx, "scripted", |_, observation| {
            if observation.height_above_target_m > 10.0 {
                Command {
                    throttle_frac: 0.2,
                    target_attitude_rad: 0.0,
                }
            } else {
                Command {
                    throttle_frac: 0.5,
                    target_attitude_rad: 0.0,
                }
            }
        })
        .unwrap();

        let replayed = replay_simulation(&ctx, "scripted", &original.actions).unwrap();

        assert_eq!(replayed.manifest, original.manifest);
        assert_eq!(replayed.events, original.events);
        assert_eq!(replayed.actions, original.actions);
    }

    #[test]
    fn neutral_physics_step_matches_the_ordinary_step_before_terminal_handling() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let command = Command {
            throttle_frac: 0.4,
            target_attitude_rad: 0.25,
        };
        let mut ordinary = SimulationState::new(&ctx).unwrap();
        let mut neutral = ordinary.clone();
        ordinary.set_command(command);
        neutral.set_command(command);

        let events = ordinary.step(&ctx);
        let contact = neutral.step_physics_and_classify_contact(&ctx);

        assert!(events.is_empty());
        assert_eq!(contact, ContactClassification::None);
        assert_eq!(ordinary.sim_time_s, neutral.sim_time_s);
        assert_eq!(ordinary.physics_step, neutral.physics_step);
        assert_eq!(ordinary.position_m, neutral.position_m);
        assert_eq!(ordinary.velocity_mps, neutral.velocity_mps);
        assert_eq!(ordinary.attitude_rad, neutral.attitude_rad);
        assert_eq!(ordinary.angular_rate_radps, neutral.angular_rate_radps);
        assert_eq!(ordinary.fuel_kg, neutral.fuel_kg);
        assert_eq!(
            ordinary.min_touchdown_clearance_m,
            neutral.min_touchdown_clearance_m
        );
        assert_eq!(ordinary.min_hull_clearance_m, neutral.min_hull_clearance_m);
        assert_eq!(ordinary.max_speed_mps, neutral.max_speed_mps);
        assert_eq!(ordinary.max_abs_attitude_rad, neutral.max_abs_attitude_rad);
        assert_eq!(
            ordinary.max_abs_angular_rate_radps,
            neutral.max_abs_angular_rate_radps
        );
        assert_eq!(ordinary.held_command, neutral.held_command);
        assert_eq!(ordinary.physical_outcome, PhysicalOutcome::Flying);
        assert_eq!(ordinary.mission_outcome, MissionOutcome::InProgress);
        assert_eq!(ordinary.end_reason, EndReason::Running);
    }

    #[test]
    fn near_edge_safe_touchdown_penetration_is_still_stable() {
        let (ctx, state) = contact_state_with_hull_penetration(0.0105, 1.5, 1.85);

        assert!(matches!(
            state.detect_contact_classification(&ctx),
            ContactClassification::StableTouchdown { on_target: true }
        ));
    }

    #[test]
    fn deeper_touchdown_penetration_is_still_a_crash() {
        let (ctx, state) = contact_state_with_hull_penetration(0.02, 1.5, 1.85);

        assert!(matches!(
            state.detect_contact_classification(&ctx),
            ContactClassification::Crash
        ));
    }

    #[test]
    fn one_step_safe_touchdown_penetration_is_still_stable() {
        let (ctx, state) = contact_state_with_hull_penetration(0.0171, 2.58, 0.758);

        assert!(matches!(
            state.detect_contact_classification(&ctx),
            ContactClassification::StableTouchdown { on_target: true }
        ));
    }

    #[test]
    fn one_step_penetration_does_not_excuse_unsafe_touchdown_speed() {
        let (ctx, state) = contact_state_with_hull_penetration(0.02, 3.286, 0.566);

        assert!(matches!(
            state.detect_contact_classification(&ctx),
            ContactClassification::Crash
        ));
    }

    #[test]
    fn landing_snapshot_uses_target_surface_frame_for_speed_and_attitude() {
        let mut scenario = smoke_scenario();
        scenario.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-10.0, -10.0), Vec2::new(10.0, 10.0)],
        };
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut state = SimulationState::new(&ctx).unwrap();
        let surface_normal = ctx
            .world
            .terrain
            .sample_surface_normal(ctx.target_pad.center_x_m);
        let surface_tangent = Vec2::new(surface_normal.y, -surface_normal.x);

        state.velocity_mps = surface_tangent * 5.0;
        state.attitude_rad = -std::f64::consts::FRAC_PI_4;

        let snapshot = state.landing_snapshot(&ctx);

        assert!(snapshot.normal_speed_mps.abs() < 1e-9);
        assert!((snapshot.tangential_speed_mps - 5.0).abs() < 1e-9);
        assert!(snapshot.attitude_error_rad.abs() < 1e-6);
    }

    #[derive(Default)]
    struct RejectingBoundedGuard {
        reject_initial: bool,
        reject_pre_transition: bool,
        pre_transition_calls: usize,
    }

    impl BoundedRunGuard for RejectingBoundedGuard {
        fn initial(
            &mut self,
            _context: &RunContext,
            _state: &SimulationState,
        ) -> Result<(), crate::BoundedGuardFailureV1> {
            if self.reject_initial {
                return Err(crate::BoundedGuardFailureV1::new(
                    BoundedGuardFailureDispositionV1::SafetyRejected,
                    "fixture initial rejection",
                ));
            }
            Ok(())
        }

        fn before_transition(
            &mut self,
            _context: &RunContext,
            _state: &SimulationState,
            _command: Command,
        ) -> Result<(), crate::BoundedGuardFailureV1> {
            self.pre_transition_calls += 1;
            if self.reject_pre_transition {
                return Err(crate::BoundedGuardFailureV1::new(
                    BoundedGuardFailureDispositionV1::SafetyRejected,
                    "fixture actuator budget rejection",
                ));
            }
            Ok(())
        }
    }

    fn bounded_contact_context(center_y_m: f64) -> RunContext {
        let mut scenario = smoke_scenario();
        scenario.initial_state = VehicleInitialState {
            position_m: Vec2::new(0.0, center_y_m),
            velocity_mps: Vec2::new(0.25, -1.5),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        RunContext::from_scenario(&scenario).unwrap()
    }

    #[test]
    fn bounded_deadlines_stop_before_an_uncovered_callback_or_extra_step() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        for (hard_end, expected_step, expected_stop, expected_coverage, expected_hard) in [
            (5, 4, BoundedRunStopCauseV1::CoverageExhausted, true, false),
            (
                3,
                3,
                BoundedRunStopCauseV1::HardDeadlineReached,
                false,
                true,
            ),
            (4, 4, BoundedRunStopCauseV1::HardDeadlineReached, true, true),
        ] {
            let mut callbacks = 0;
            let mut guard = RejectingBoundedGuard::default();
            let result = run_simulation_bounded(
                &ctx,
                "bounded_fixture",
                BoundedRunLimitsV1 {
                    command_coverage_end_physics_step: 4,
                    hard_end_physics_step: hard_end,
                },
                |_, _| {
                    callbacks += 1;
                    Ok(Command::idle())
                },
                &mut guard,
            )
            .unwrap();

            assert_eq!(result.final_state.physics_step, expected_step);
            assert_eq!(result.run.manifest.physics_steps, expected_step);
            assert_eq!(result.stop, expected_stop);
            assert_eq!(result.coverage_reached, expected_coverage);
            assert_eq!(result.hard_end_reached, expected_hard);
            assert_eq!(callbacks, 2, "no callback is allowed at or after the bound");
            assert_eq!(result.run.actions.len(), 2);
            assert_eq!(guard.pre_transition_calls, expected_step as usize);
            assert_eq!(result.run.manifest.end_reason, EndReason::Running);
            assert!(result.run.events.iter().all(|event| {
                event.kind != EventKind::MissionEnded && event.kind != EventKind::Crash
            }));
        }
    }

    #[test]
    fn bounded_contact_keeps_incoming_state_before_stable_normalization() {
        let ctx = bounded_contact_context(3.21);
        let mut callbacks = 0;
        let mut guard = AllowAllBoundedRunGuard;
        let result = run_simulation_bounded(
            &ctx,
            "bounded_contact_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 2,
                hard_end_physics_step: 1,
            },
            |_, _| {
                callbacks += 1;
                Ok(Command::idle())
            },
            &mut guard,
        )
        .unwrap();

        let incoming = result.incoming_contact.as_ref().unwrap();
        assert!(matches!(
            incoming.classification,
            ContactClassification::StableTouchdown { on_target: true }
        ));
        assert_eq!(incoming.state.physics_step, 1);
        assert!(incoming.state.velocity_mps.y < -1.0);
        assert!(incoming.state.velocity_mps.x > 0.0);
        assert_eq!(result.final_state.velocity_mps, Vec2::new(0.0, 0.0));
        assert_eq!(result.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert!(result.hard_end_reached);
        assert!(!result.coverage_reached);
        assert_eq!(result.run.manifest.physics_steps, 1);
        assert_eq!(callbacks, 1);
    }

    #[test]
    fn contact_on_a_tied_hard_and_coverage_boundary_wins_before_driver_stop() {
        let ctx = bounded_contact_context(3.225);
        let mut callbacks = 0;
        let mut guard = AllowAllBoundedRunGuard;
        let result = run_simulation_bounded(
            &ctx,
            "bounded_contact_tie_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 2,
                hard_end_physics_step: 2,
            },
            |_, _| {
                callbacks += 1;
                Ok(Command::idle())
            },
            &mut guard,
        )
        .unwrap();

        assert_eq!(result.final_state.physics_step, 2);
        assert!(result.incoming_contact.is_some());
        assert_eq!(result.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert!(result.coverage_reached && result.hard_end_reached);
        assert_eq!(callbacks, 1, "contact prevents a tick-2 callback");
    }

    #[test]
    fn bounded_initial_and_pretransition_rejections_keep_truthful_prefixes() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let limits = BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 4,
            hard_end_physics_step: 8,
        };

        let mut initial_guard = RejectingBoundedGuard {
            reject_initial: true,
            ..RejectingBoundedGuard::default()
        };
        let initial = run_simulation_bounded(
            &ctx,
            "bounded_initial_reject",
            limits,
            |_, _| Ok(Command::idle()),
            &mut initial_guard,
        )
        .unwrap();
        assert_eq!(initial.stop, BoundedRunStopCauseV1::SafetyRejected);
        assert_eq!(initial.final_state.physics_step, 0);
        assert!(initial.run.actions.is_empty());
        assert!(initial.run.events.is_empty());
        assert_eq!(
            initial.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::InitialGuard
        );

        let mut guard = RejectingBoundedGuard {
            reject_pre_transition: true,
            ..RejectingBoundedGuard::default()
        };
        let pre_transition = run_simulation_bounded(
            &ctx,
            "bounded_pre_reject",
            limits,
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();
        assert_eq!(pre_transition.stop, BoundedRunStopCauseV1::SafetyRejected);
        assert_eq!(pre_transition.final_state.physics_step, 0);
        assert_eq!(pre_transition.run.actions.len(), 1);
        assert_eq!(pre_transition.run.events.len(), 1);
        assert_eq!(pre_transition.run.manifest.end_reason, EndReason::Running);
        assert_eq!(
            pre_transition.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::PreTransitionGuard
        );
    }

    #[test]
    fn callback_and_nonfinite_transition_failures_keep_serializable_finite_prefixes() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let mut guard = AllowAllBoundedRunGuard;
        let callback_failure = run_simulation_bounded(
            &ctx,
            "callback_failure_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            |_, _| Err("fixture command selection failure".to_owned()),
            &mut guard,
        )
        .unwrap();
        assert_eq!(
            callback_failure.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(callback_failure.final_state.physics_step, 0);
        assert!(callback_failure.final_state.sim_time_s.is_finite());
        assert!(callback_failure.run.actions.is_empty());
        assert_eq!(callback_failure.run.samples.len(), 1);
        assert_eq!(
            callback_failure.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::CommandSelection
        );

        let mut guard = AllowAllBoundedRunGuard;
        let nonfinite_command = run_simulation_bounded(
            &ctx,
            "nonfinite_command_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            |_, _| {
                Ok(Command {
                    throttle_frac: f64::NAN,
                    target_attitude_rad: 0.0,
                })
            },
            &mut guard,
        )
        .unwrap();
        let command_failure = nonfinite_command.failure.as_ref().unwrap();
        assert_eq!(
            nonfinite_command.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(
            command_failure.stage,
            BoundedRunFailureStageV1::CommandSelection
        );
        assert_eq!(
            command_failure.non_finite.as_ref().unwrap().field,
            "command.throttle_frac"
        );
        assert_eq!(
            command_failure.non_finite.as_ref().unwrap().category,
            BoundedRunNonFiniteCategoryV1::Nan
        );
        assert!(nonfinite_command.run.actions.is_empty());
        let json = serde_json::to_string(&nonfinite_command).unwrap();
        assert!(serde_json::from_str::<BoundedRunArtifactsV1>(&json).is_ok());

        let mut nonfinite_projection_scenario = smoke_scenario();
        nonfinite_projection_scenario.vehicle.dry_mass_kg = 1.0e308;
        nonfinite_projection_scenario.vehicle.initial_fuel_kg = 1.0e308;
        nonfinite_projection_scenario.vehicle.max_fuel_kg = 1.0e308;
        let nonfinite_projection_context =
            RunContext::from_scenario(&nonfinite_projection_scenario).unwrap();
        let mut callback_called = false;
        let mut guard = AllowAllBoundedRunGuard;
        let projection_error = run_simulation_bounded(
            &nonfinite_projection_context,
            "nonfinite_initial_projection_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            |_, _| {
                callback_called = true;
                Ok(Command::idle())
            },
            &mut guard,
        )
        .unwrap_err();
        assert!(projection_error.to_string().contains("observation.mass_kg"));
        assert!(!callback_called);

        let mut extreme_scenario = smoke_scenario();
        extreme_scenario.world.gravity_mps2 = 1.0e308;
        let extreme_context = RunContext::from_scenario(&extreme_scenario).unwrap();
        let mut guard = AllowAllBoundedRunGuard;
        let numeric_failure = run_simulation_bounded(
            &extreme_context,
            "nonfinite_transition_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();
        let failure = numeric_failure.failure.as_ref().unwrap();
        assert_eq!(
            numeric_failure.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(failure.stage, BoundedRunFailureStageV1::PhysicsTransition);
        assert_eq!(
            failure.disposition,
            BoundedGuardFailureDispositionV1::ExecutionInvalid
        );
        assert_eq!(failure.boundary_physics_step, 1);
        assert_eq!(failure.non_finite.as_ref().unwrap().field, "max_speed_mps");
        assert_eq!(
            failure.non_finite.as_ref().unwrap().category,
            BoundedRunNonFiniteCategoryV1::PositiveInfinity
        );
        assert_eq!(numeric_failure.final_state.physics_step, 0);
        assert_eq!(numeric_failure.run.manifest.end_reason, EndReason::Running);
        assert!(numeric_failure.final_state.sim_time_s.is_finite());
        assert!(numeric_failure.final_state.position_m.x.is_finite());
        assert!(numeric_failure.final_state.position_m.y.is_finite());
        assert!(numeric_failure.final_state.max_speed_mps.is_finite());
        assert_eq!(numeric_failure.run.samples.len(), 1);
        let json = serde_json::to_string(&numeric_failure).unwrap();
        assert!(serde_json::from_str::<BoundedRunArtifactsV1>(&json).is_ok());
    }

    #[test]
    fn finite_zero_fuel_idle_coast_is_not_a_core_terminal_condition() {
        let mut scenario = smoke_scenario();
        scenario.vehicle.initial_fuel_kg = 1.0e-6;
        scenario.initial_state.position_m.y = 100.0;
        let ctx = RunContext::from_scenario(&scenario).unwrap();
        let mut guard = AllowAllBoundedRunGuard;
        let result = run_simulation_bounded(
            &ctx,
            "zero_fuel_coast_fixture",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            |_, observation| {
                Ok(if observation.physics_step == 0 {
                    Command {
                        throttle_frac: 1.0,
                        target_attitude_rad: 0.0,
                    }
                } else {
                    Command::idle()
                })
            },
            &mut guard,
        )
        .unwrap();

        assert_eq!(result.stop, BoundedRunStopCauseV1::CoverageExhausted);
        assert_eq!(result.final_state.physics_step, 4);
        assert_eq!(result.final_state.fuel_kg, 0.0);
        assert_eq!(result.run.manifest.end_reason, EndReason::Running);
    }

    #[test]
    fn ordinary_horizon_is_real_terminal_but_contact_on_horizon_wins() {
        let mut airborne_scenario = smoke_scenario();
        airborne_scenario.sim.max_time_s = 10.0;
        airborne_scenario.initial_state.position_m.y = 100.0;
        airborne_scenario.initial_state.velocity_mps = Vec2::new(0.0, 0.0);
        let airborne_context = RunContext::from_scenario(&airborne_scenario).unwrap();
        let horizon_step = 1200;
        let limits = BoundedRunLimitsV1 {
            command_coverage_end_physics_step: horizon_step,
            hard_end_physics_step: horizon_step,
        };
        let mut guard = AllowAllBoundedRunGuard;
        let horizon = run_simulation_bounded(
            &airborne_context,
            "horizon_precedence_fixture",
            limits,
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();
        assert_eq!(horizon.stop, BoundedRunStopCauseV1::ScenarioHorizonReached);
        assert_eq!(horizon.final_state.physics_step, horizon_step);
        assert_eq!(horizon.run.manifest.end_reason, EndReason::MaxTimeReached);
        assert!(horizon.coverage_reached && horizon.hard_end_reached);

        let mut contact_scenario = smoke_scenario();
        contact_scenario.sim.max_time_s = 10.0;
        let dt_s = 1.0 / f64::from(contact_scenario.sim.physics_hz);
        let steps = horizon_step as f64;
        contact_scenario.initial_state.position_m.y = 3.1999;
        contact_scenario.initial_state.velocity_mps.y =
            contact_scenario.world.gravity_mps2 * dt_s * (steps + 1.0) * 0.5;
        let contact_context = RunContext::from_scenario(&contact_scenario).unwrap();
        let mut guard = AllowAllBoundedRunGuard;
        let contact = run_simulation_bounded(
            &contact_context,
            "contact_at_horizon_fixture",
            limits,
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();
        assert_eq!(contact.final_state.physics_step, horizon_step);
        assert_eq!(contact.stop, BoundedRunStopCauseV1::MissionTerminal);
        assert_eq!(contact.run.manifest.end_reason, EndReason::Crash);
        assert!(matches!(
            contact
                .incoming_contact
                .as_ref()
                .map(|incoming| &incoming.classification),
            Some(ContactClassification::Crash)
        ));
        assert!(contact.coverage_reached && contact.hard_end_reached);
    }

    #[test]
    fn bounded_replay_rejects_truncation_extra_actions_and_offclock_payloads() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let limits = BoundedRunLimitsV1 {
            command_coverage_end_physics_step: 4,
            hard_end_physics_step: 8,
        };
        let mut allow = AllowAllBoundedRunGuard;
        let original = run_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            limits,
            |_, _| Ok(Command::idle()),
            &mut allow,
        )
        .unwrap();
        assert_eq!(original.run.actions.len(), 2);

        let mut replay_guard = AllowAllBoundedRunGuard;
        let exact = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &original.run.actions,
            limits,
            &mut replay_guard,
        )
        .unwrap();
        assert_eq!(exact.run, original.run);
        assert_eq!(exact.final_state, original.final_state);
        assert_eq!(exact.stop, original.stop);

        let mut truncated_guard = AllowAllBoundedRunGuard;
        let truncated = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &original.run.actions[..1],
            limits,
            &mut truncated_guard,
        )
        .unwrap();
        assert_eq!(truncated.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(
            truncated.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
        assert_eq!(truncated.run.manifest.physics_steps, 2);
        assert_eq!(truncated.run.actions.len(), 1);

        let mut extra_actions = original.run.actions.clone();
        extra_actions.push(ActionLogEntry {
            sim_time_s: 4.0 / f64::from(ctx.sim.physics_hz),
            physics_step: 4,
            controller_update_index: 2,
            command: Command::idle(),
        });
        let mut extra_guard = AllowAllBoundedRunGuard;
        let extra = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &extra_actions,
            limits,
            &mut extra_guard,
        )
        .unwrap();
        assert_eq!(extra.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(extra.run.manifest.physics_steps, 4);

        let mut offclock_actions = original.run.actions.clone();
        offclock_actions[0].physics_step = 1;
        let mut offclock_guard = AllowAllBoundedRunGuard;
        let offclock = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &offclock_actions,
            limits,
            &mut offclock_guard,
        )
        .unwrap();
        assert_eq!(offclock.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(
            offclock.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
        assert!(offclock.run.actions.is_empty());

        let mut nonfinite_timestamp_actions = original.run.actions.clone();
        nonfinite_timestamp_actions[0].sim_time_s = f64::NAN;
        let mut nonfinite_timestamp_guard = AllowAllBoundedRunGuard;
        let nonfinite_timestamp = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &nonfinite_timestamp_actions,
            limits,
            &mut nonfinite_timestamp_guard,
        )
        .unwrap();
        let timestamp_failure = nonfinite_timestamp.failure.as_ref().unwrap();
        assert_eq!(
            nonfinite_timestamp.stop,
            BoundedRunStopCauseV1::ExecutionInvalid
        );
        assert_eq!(
            timestamp_failure.non_finite.as_ref().unwrap().field,
            "action.sim_time_s"
        );
        assert_eq!(
            timestamp_failure.non_finite.as_ref().unwrap().category,
            BoundedRunNonFiniteCategoryV1::Nan
        );
        assert!(nonfinite_timestamp.run.actions.is_empty());

        let mut missing_guard = AllowAllBoundedRunGuard;
        let missing = replay_simulation_bounded(
            &ctx,
            "bounded_replay_fixture",
            &[],
            limits,
            &mut missing_guard,
        )
        .unwrap();
        assert_eq!(missing.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(missing.final_state.physics_step, 0);
        assert_eq!(
            missing.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
    }

    #[test]
    fn extra_replay_actions_after_initial_safety_stop_are_still_invalid() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let actions = [ActionLogEntry {
            sim_time_s: 0.0,
            physics_step: 0,
            controller_update_index: 0,
            command: Command::idle(),
        }];
        let mut empty_guard = RejectingBoundedGuard {
            reject_initial: true,
            ..RejectingBoundedGuard::default()
        };
        let empty = replay_simulation_bounded(
            &ctx,
            "initial_stop_replay",
            &[],
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            &mut empty_guard,
        )
        .unwrap();
        assert_eq!(empty.stop, BoundedRunStopCauseV1::SafetyRejected);
        assert_eq!(
            empty.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::InitialGuard
        );
        assert!(empty.run.actions.is_empty());

        let mut guard = RejectingBoundedGuard {
            reject_initial: true,
            ..RejectingBoundedGuard::default()
        };
        let result = replay_simulation_bounded(
            &ctx,
            "initial_stop_replay",
            &actions,
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 4,
                hard_end_physics_step: 8,
            },
            &mut guard,
        )
        .unwrap();

        assert_eq!(result.stop, BoundedRunStopCauseV1::ExecutionInvalid);
        assert_eq!(
            result.failure.as_ref().unwrap().stage,
            BoundedRunFailureStageV1::ReplayInput
        );
        assert!(
            result
                .failure
                .as_ref()
                .unwrap()
                .reason
                .contains("fixture initial rejection")
        );
        assert!(result.run.actions.is_empty());
        assert_eq!(result.final_state.physics_step, 0);
    }

    #[test]
    fn ordinary_and_bounded_terminal_runs_keep_identical_legacy_artifacts() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let ordinary = run_simulation(&ctx, "legacy-parity", |_, _| Command::idle()).unwrap();
        let mut guard = AllowAllBoundedRunGuard;
        let bounded = run_simulation_bounded(
            &ctx,
            "legacy-parity",
            BoundedRunLimitsV1 {
                command_coverage_end_physics_step: 1200,
                hard_end_physics_step: 1200,
            },
            |_, _| Ok(Command::idle()),
            &mut guard,
        )
        .unwrap();

        assert_eq!(bounded.run, ordinary);
        assert_eq!(bounded.stop, BoundedRunStopCauseV1::MissionTerminal);
    }
}
