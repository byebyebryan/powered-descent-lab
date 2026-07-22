use super::*;

pub(super) fn validate_pack(pack: &ScenarioPackSpec) -> Result<()> {
    if pack.id.trim().is_empty() {
        bail!("pack id must not be empty");
    }
    if pack.name.trim().is_empty() {
        bail!("pack name must not be empty");
    }
    if pack.entries.is_empty() {
        bail!("pack must contain at least one entry");
    }
    if let Some(max_time_s) = pack.terminal_matrix_max_time_s
        && (!max_time_s.is_finite() || max_time_s <= 0.0)
    {
        bail!("terminal_matrix_max_time_s must be finite and > 0");
    }

    let mut seen_ids = BTreeSet::new();
    for entry in &pack.entries {
        if entry.id().trim().is_empty() {
            bail!("pack entry id must not be empty");
        }
        if !seen_ids.insert(entry.id().to_owned()) {
            bail!("duplicate pack entry id '{}'", entry.id());
        }

        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                if entry.controller.trim().is_empty() {
                    bail!("pack entry controller must not be empty");
                }
                if entry.scenario.trim().is_empty() {
                    bail!("pack entry scenario path must not be empty");
                }
            }
            ScenarioPackEntry::Family(entry) => validate_family_entry(entry)?,
            ScenarioPackEntry::TerminalMatrix(entry) => validate_terminal_matrix_entry(entry)?,
            ScenarioPackEntry::TransferMatrix(entry) => validate_transfer_matrix_entry(entry)?,
        }
    }

    Ok(())
}

pub(super) fn validate_family_entry(entry: &ScenarioFamilyEntry) -> Result<()> {
    if entry.controller.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty controller",
            entry.id
        );
    }
    if entry.family.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty family id",
            entry.id
        );
    }
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "family entry '{}' must define a non-empty base_scenario path",
            entry.id
        );
    }
    if entry.tags.iter().any(|tag| tag.trim().is_empty()) {
        bail!(
            "family entry '{}' tags must not contain empty values",
            entry.id
        );
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "family entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }

    let explicit_seed_count = usize::from(!entry.seeds.is_empty());
    let range_seed_count = usize::from(entry.seed_range.is_some());
    if explicit_seed_count + range_seed_count != 1 {
        bail!(
            "family entry '{}' must define exactly one of 'seeds' or 'seed_range'",
            entry.id
        );
    }
    if let Some(seed_range) = &entry.seed_range
        && seed_range.count == 0
    {
        bail!("family entry '{}' seed_range.count must be > 0", entry.id);
    }
    if !entry.seeds.is_empty() {
        let mut seen = BTreeSet::new();
        for seed in &entry.seeds {
            if !seen.insert(*seed) {
                bail!(
                    "family entry '{}' seeds must not contain duplicates (duplicate seed {})",
                    entry.id,
                    seed
                );
            }
        }
    }

    let mut seen_ids = BTreeSet::new();
    for perturbation in &entry.perturbations {
        validate_numeric_perturbation(entry, perturbation, &mut seen_ids)?;
    }

    Ok(())
}

pub(super) fn validate_terminal_matrix_entry(entry: &TerminalMatrixEntry) -> Result<()> {
    if entry.terminal_matrix.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty terminal_matrix",
            entry.id
        );
    }
    let family_spec = terminal_arrival_family_spec(&entry.terminal_matrix)?;
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty base_scenario",
            entry.id
        );
    }
    if entry.condition_set.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty condition_set",
            entry.id
        );
    }
    terminal_condition_spec(&entry.condition_set)?;
    if entry.vehicle_variant.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty vehicle_variant",
            entry.id
        );
    }
    if entry.expectation_tier.trim().is_empty() {
        bail!(
            "terminal matrix entry '{}' must define a non-empty expectation_tier",
            entry.id
        );
    }
    if entry.lanes.is_empty() {
        bail!(
            "terminal matrix entry '{}' must define at least one lane",
            entry.id
        );
    }
    let mut seen_lane_ids = BTreeSet::new();
    for lane in &entry.lanes {
        if lane.id.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' has a lane with an empty id",
                entry.id
            );
        }
        if lane.controller.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' lane '{}' must define a controller",
                entry.id,
                lane.id
            );
        }
        if !seen_lane_ids.insert(lane.id.clone()) {
            bail!(
                "terminal matrix entry '{}' has duplicate lane id '{}'",
                entry.id,
                lane.id
            );
        }
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }
    let mut seen_arc_points = BTreeSet::new();
    for arc_point in &entry.arc_points {
        if arc_point.trim().is_empty() {
            bail!(
                "terminal matrix entry '{}' has an empty arc_point selector",
                entry.id
            );
        }
        if !seen_arc_points.insert(arc_point.clone()) {
            bail!(
                "terminal matrix entry '{}' has duplicate arc_point selector '{}'",
                entry.id,
                arc_point
            );
        }
        if !family_spec
            .arc_points
            .iter()
            .any(|candidate| candidate.id == arc_point)
        {
            bail!(
                "terminal matrix entry '{}' arc_point selector '{}' is not supported by matrix '{}'",
                entry.id,
                arc_point,
                entry.terminal_matrix
            );
        }
    }
    let mut seen_adjustment_ids = BTreeSet::new();
    for adjustment in &entry.adjustments {
        validate_numeric_adjustment(
            &entry.id,
            "terminal matrix",
            adjustment,
            &mut seen_adjustment_ids,
        )?;
    }
    Ok(())
}

pub(super) fn validate_numeric_adjustment(
    entry_id: &str,
    entry_kind: &str,
    adjustment: &NumericAdjustmentSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if adjustment.id.trim().is_empty() {
        bail!(
            "{entry_kind} entry '{}' has an adjustment with an empty id",
            entry_id
        );
    }
    if !seen_ids.insert(adjustment.id.clone()) {
        bail!(
            "{entry_kind} entry '{}' has duplicate adjustment id '{}'",
            entry_id,
            adjustment.id
        );
    }
    if !is_supported_terminal_adjustment_path(&adjustment.path) {
        bail!(
            "{entry_kind} entry '{}' adjustment '{}' uses unsupported path '{}'",
            entry_id,
            adjustment.id,
            adjustment.path
        );
    }
    if !adjustment.value.is_finite() {
        bail!(
            "{entry_kind} entry '{}' adjustment '{}' must be finite",
            entry_id,
            adjustment.id
        );
    }
    Ok(())
}

pub(super) fn validate_transfer_matrix_entry(entry: &TransferMatrixEntry) -> Result<()> {
    if entry.transfer_matrix.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty transfer_matrix",
            entry.id
        );
    }
    let route_spec = transfer_route_family_spec(&entry.transfer_matrix)?;
    if entry.base_scenario.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty base_scenario",
            entry.id
        );
    }
    if entry.vehicle_variant.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty vehicle_variant",
            entry.id
        );
    }
    if entry.expectation_tier.trim().is_empty() {
        bail!(
            "transfer matrix entry '{}' must define a non-empty expectation_tier",
            entry.id
        );
    }
    if let Some(profile) = &entry.waypoint_profile {
        validate_transfer_waypoint_profile(&entry.id, profile)?;
        if profile == TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
            && entry.expectation_tier != TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
        {
            bail!(
                "transfer matrix entry '{}' waypoint_profile '{}' requires expectation_tier '{}'",
                entry.id,
                profile,
                TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
            );
        }
        if profile == TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
            && entry.expectation_tier != TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
        {
            bail!(
                "transfer matrix entry '{}' waypoint_profile '{}' requires expectation_tier '{}'",
                entry.id,
                profile,
                TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC
            );
        }
    }
    if let Some(envelope) = &entry.waypoint_handoff_envelope {
        validate_transfer_waypoint_envelope(&entry.id, envelope)?;
        if entry.waypoint_profile.is_none() {
            bail!(
                "transfer matrix entry '{}' waypoint_handoff_envelope requires waypoint_profile",
                entry.id
            );
        }
    }
    if matches!(
        entry.evaluation_goal,
        TransferMatrixEvaluationGoal::WaypointHandoff
            | TransferMatrixEvaluationGoal::WaypointSequence
    ) && entry.waypoint_profile.is_none()
    {
        bail!(
            "transfer matrix entry '{}' evaluation_goal '{}' requires waypoint_profile",
            entry.id,
            entry.evaluation_goal.as_str()
        );
    }
    if entry.lanes.is_empty() {
        bail!(
            "transfer matrix entry '{}' must define at least one lane",
            entry.id
        );
    }
    let mut seen_lane_ids = BTreeSet::new();
    for lane in &entry.lanes {
        if lane.id.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has a lane with an empty id",
                entry.id
            );
        }
        if lane.controller.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' lane '{}' must define a controller",
                entry.id,
                lane.id
            );
        }
        if !seen_lane_ids.insert(lane.id.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate lane id '{}'",
                entry.id,
                lane.id
            );
        }
    }
    for (key, value) in &entry.metadata {
        if key.trim().is_empty() || value.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' metadata keys and values must not be empty",
                entry.id
            );
        }
    }
    let mut seen_route_angles = BTreeSet::new();
    for route_angle in &entry.route_angles {
        if route_angle.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has an empty route_angle selector",
                entry.id
            );
        }
        if !seen_route_angles.insert(route_angle.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate route_angle selector '{}'",
                entry.id,
                route_angle
            );
        }
        if !route_spec
            .route_angles
            .iter()
            .any(|candidate| candidate.id == route_angle)
        {
            bail!(
                "transfer matrix entry '{}' route_angle selector '{}' is not supported by matrix '{}'",
                entry.id,
                route_angle,
                entry.transfer_matrix
            );
        }
    }
    let mut seen_radius_tiers = BTreeSet::new();
    for radius_tier in &entry.radius_tiers {
        if radius_tier.trim().is_empty() {
            bail!(
                "transfer matrix entry '{}' has an empty radius_tier selector",
                entry.id
            );
        }
        if !seen_radius_tiers.insert(radius_tier.clone()) {
            bail!(
                "transfer matrix entry '{}' has duplicate radius_tier selector '{}'",
                entry.id,
                radius_tier
            );
        }
        if !route_spec
            .radius_tiers
            .iter()
            .any(|candidate| candidate.id == radius_tier)
        {
            bail!(
                "transfer matrix entry '{}' radius_tier selector '{}' is not supported by matrix '{}'",
                entry.id,
                radius_tier,
                entry.transfer_matrix
            );
        }
    }
    let mut seen_adjustment_ids = BTreeSet::new();
    for adjustment in &entry.adjustments {
        validate_numeric_adjustment(
            &entry.id,
            "transfer matrix",
            adjustment,
            &mut seen_adjustment_ids,
        )?;
    }
    Ok(())
}

pub(super) fn validate_transfer_waypoint_profile(entry_id: &str, profile: &str) -> Result<()> {
    if profile.trim().is_empty() {
        bail!("transfer matrix entry '{entry_id}' waypoint_profile must not be empty");
    }
    if !matches!(
        profile,
        TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1
            | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
    ) {
        bail!(
            "transfer matrix entry '{}' waypoint_profile '{}' is not supported",
            entry_id,
            profile
        );
    }
    Ok(())
}

pub(super) fn validate_transfer_waypoint_envelope(entry_id: &str, envelope: &str) -> Result<()> {
    if envelope.trim().is_empty() {
        bail!("transfer matrix entry '{entry_id}' waypoint_handoff_envelope must not be empty");
    }
    if !matches!(
        envelope,
        TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1
            | TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1
            | TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
            | TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
    ) {
        bail!(
            "transfer matrix entry '{}' waypoint_handoff_envelope '{}' is not supported",
            entry_id,
            envelope
        );
    }
    Ok(())
}

pub(super) fn validate_numeric_perturbation(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seen_ids: &mut BTreeSet<String>,
) -> Result<()> {
    if perturbation.id.trim().is_empty() {
        bail!(
            "family entry '{}' has a perturbation with an empty id",
            entry.id
        );
    }
    if !seen_ids.insert(perturbation.id.clone()) {
        bail!(
            "family entry '{}' has duplicate perturbation id '{}'",
            entry.id,
            perturbation.id
        );
    }
    if !is_supported_numeric_path(&perturbation.path) {
        bail!(
            "family entry '{}' perturbation '{}' uses unsupported path '{}'",
            entry.id,
            perturbation.id,
            perturbation.path
        );
    }
    if !perturbation.min.is_finite() || !perturbation.max.is_finite() {
        bail!(
            "family entry '{}' perturbation '{}' bounds must be finite",
            entry.id,
            perturbation.id
        );
    }
    if perturbation.max < perturbation.min {
        bail!(
            "family entry '{}' perturbation '{}' max must be >= min",
            entry.id,
            perturbation.id
        );
    }
    if let Some(step) = perturbation.quantize
        && (!step.is_finite() || step <= 0.0)
    {
        bail!(
            "family entry '{}' perturbation '{}' quantize must be > 0",
            entry.id,
            perturbation.id
        );
    }
    Ok(())
}

pub(super) fn resolve_pack_runs(
    pack: &ScenarioPackSpec,
    base_dir: &Path,
) -> Result<Vec<ResolvedBatchRun>> {
    let mut resolved = Vec::new();
    for entry in &pack.entries {
        match entry {
            ScenarioPackEntry::Scenario(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.push(resolve_concrete_run(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::Family(entry) => {
                let controller_spec = load_controller_spec(
                    base_dir,
                    entry.controller.as_str(),
                    entry.controller_config.as_deref(),
                )?;
                resolved.extend(resolve_family_runs(entry, base_dir, &controller_spec)?)
            }
            ScenarioPackEntry::TerminalMatrix(entry) => resolved.extend(
                resolve_terminal_matrix_runs(entry, base_dir, pack.terminal_matrix_max_time_s)?,
            ),
            ScenarioPackEntry::TransferMatrix(entry) => {
                resolved.extend(resolve_transfer_matrix_runs(entry, base_dir)?)
            }
        }
    }

    let mut seen_run_ids = BTreeSet::new();
    for run in &resolved {
        if !seen_run_ids.insert(run.descriptor.run_id.clone()) {
            bail!(
                "resolved pack contains duplicate run id '{}'",
                run.descriptor.run_id
            );
        }
    }
    Ok(resolved)
}

pub(super) fn resolve_concrete_run(
    entry: &ConcreteScenarioPackEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<ResolvedBatchRun> {
    let scenario_path = base_dir.join(&entry.scenario);
    let mut scenario = load_scenario(&scenario_path)?;
    scenario.metadata.extend(entry.metadata.clone());
    let family_id = scenario.metadata.get("family").cloned();
    let selector = selector_axes_from_metadata(&scenario.metadata);
    let descriptor = ResolvedRunDescriptor {
        run_id: sanitize_token(&entry.id),
        entry_id: entry.id.clone(),
        source_kind: ResolvedRunSourceKind::ConcreteScenario,
        scenario_source: entry.scenario.clone(),
        resolved_scenario_id: scenario.id.clone(),
        resolved_scenario_name: scenario.name.clone(),
        family_id,
        selector,
        lane_id: entry.controller.clone(),
        resolved_seed: scenario.seed,
        resolved_parameters: BTreeMap::new(),
        controller_id: controller_spec.id().to_owned(),
        controller_spec: controller_spec.clone(),
    };

    Ok(ResolvedBatchRun {
        descriptor,
        scenario,
    })
}

pub(super) fn resolve_family_runs(
    entry: &ScenarioFamilyEntry,
    base_dir: &Path,
    controller_spec: &ControllerSpec,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let mut runs = Vec::new();

    for seed in family_entry_seeds(entry) {
        let (scenario, resolved_parameters) = resolve_family_scenario(entry, &base_scenario, seed)?;
        let selector = selector_axes_from_metadata(&scenario.metadata);
        let descriptor = ResolvedRunDescriptor {
            run_id: resolved_family_run_id(&entry.id, seed),
            entry_id: entry.id.clone(),
            source_kind: ResolvedRunSourceKind::FamilySweep,
            scenario_source: entry.base_scenario.clone(),
            resolved_scenario_id: scenario.id.clone(),
            resolved_scenario_name: scenario.name.clone(),
            family_id: Some(entry.family.clone()),
            selector,
            lane_id: entry.controller.clone(),
            resolved_seed: seed,
            resolved_parameters,
            controller_id: controller_spec.id().to_owned(),
            controller_spec: controller_spec.clone(),
        };
        runs.push(ResolvedBatchRun {
            descriptor,
            scenario,
        });
    }

    Ok(runs)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalArcPointSpec {
    id: &'static str,
    angle_deg: f64,
    nominal_ttg_s: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalArrivalFamilySpec {
    arrival_family: &'static str,
    gravity_mps2: f64,
    radius_nominal_m: f64,
    low_multiplier: f64,
    high_multiplier: f64,
    clamp_low_to_descending: bool,
    arc_points: &'static [TerminalArcPointSpec],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalBandSpec {
    id: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalSeedSpec {
    index: u64,
    radial_pct: Option<f64>,
    speed_pct: Option<f64>,
    error_level_index: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferRouteAngleSpec {
    id: &'static str,
    angle_deg: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferRouteFamilySpec {
    route_family: &'static str,
    gravity_mps2: f64,
    default_radius_tier: &'static str,
    radius_tiers: &'static [TransferRadiusTierSpec],
    route_angles: &'static [TransferRouteAngleSpec],
    smoke_route_angles: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferRadiusTierSpec {
    id: &'static str,
    radius_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransferSeedSpec {
    index: u64,
    radius_pct: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalProjectedErrorKind {
    Undershoot,
    Overshoot,
}

impl TerminalProjectedErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Undershoot => "undershoot",
            Self::Overshoot => "overshoot",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalProjectedErrorSpec {
    kind: TerminalProjectedErrorKind,
    severity: &'static str,
    magnitudes_m: [f64; 3],
}

#[derive(Clone, Copy, Debug)]
pub(super) enum TerminalReactiveTerrainHazard {
    ContainmentBackstop,
    DescentClip,
}

impl TerminalReactiveTerrainHazard {
    fn as_str(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "containment_backstop",
            Self::DescentClip => "descent_clip",
        }
    }

    fn obstacle_kind(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "backstop",
            Self::DescentClip => "shoulder",
        }
    }

    fn obstacle_placement(self) -> &'static str {
        match self {
            Self::ContainmentBackstop => "target_side",
            Self::DescentClip => "terminal_approach",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalReactiveTerrainSpec {
    hazard: TerminalReactiveTerrainHazard,
    variant: &'static str,
    height_offset_m: f64,
    pad_clearance_gap_m: f64,
    shoulder_width_m: f64,
    top_width_m: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum TerminalConditionSpec {
    Clean,
    ProjectedError(TerminalProjectedErrorSpec),
    ReactiveTerrain(TerminalReactiveTerrainSpec),
}

impl TerminalConditionSpec {
    fn kind_label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::ProjectedError(_) => "projected_error",
            Self::ReactiveTerrain(_) => "reactive_terrain",
        }
    }
}

pub(super) const HALF_ARC_TERMINAL_V1_ARC_POINTS: [TerminalArcPointSpec; 7] = [
    TerminalArcPointSpec {
        id: "a00",
        angle_deg: 0.0,
        nominal_ttg_s: 9.00,
    },
    TerminalArcPointSpec {
        id: "a15",
        angle_deg: 15.0,
        nominal_ttg_s: 9.00,
    },
    TerminalArcPointSpec {
        id: "a30",
        angle_deg: 30.0,
        nominal_ttg_s: 8.75,
    },
    TerminalArcPointSpec {
        id: "a45",
        angle_deg: 45.0,
        nominal_ttg_s: 8.50,
    },
    TerminalArcPointSpec {
        id: "a60",
        angle_deg: 60.0,
        nominal_ttg_s: 8.25,
    },
    TerminalArcPointSpec {
        id: "a70",
        angle_deg: 70.0,
        nominal_ttg_s: 8.00,
    },
    TerminalArcPointSpec {
        id: "a80",
        angle_deg: 80.0,
        nominal_ttg_s: 8.00,
    },
];

pub(super) const HALF_ARC_TERMINAL_V1_SPEC: TerminalArrivalFamilySpec = TerminalArrivalFamilySpec {
    arrival_family: "half_arc_terminal_v1",
    gravity_mps2: 9.81,
    radius_nominal_m: 800.0,
    low_multiplier: 1.25,
    high_multiplier: 0.75,
    clamp_low_to_descending: false,
    arc_points: &HALF_ARC_TERMINAL_V1_ARC_POINTS,
};

pub(super) const TERMINAL_BANDS: [TerminalBandSpec; 3] = [
    TerminalBandSpec { id: "low" },
    TerminalBandSpec { id: "mid" },
    TerminalBandSpec { id: "high" },
];

pub(super) const TERMINAL_SMOKE_SEEDS: [TerminalSeedSpec; 3] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
        error_level_index: 2,
    },
];

pub(super) const TERMINAL_FULL_SEEDS: [TerminalSeedSpec; 12] = [
    TerminalSeedSpec {
        index: 0,
        radial_pct: Some(0.015),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 1,
        radial_pct: Some(-0.015),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 2,
        radial_pct: Some(0.030),
        speed_pct: None,
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 3,
        radial_pct: Some(-0.030),
        speed_pct: None,
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 4,
        radial_pct: Some(0.045),
        speed_pct: None,
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 5,
        radial_pct: Some(-0.045),
        speed_pct: None,
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 6,
        radial_pct: None,
        speed_pct: Some(0.010),
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 7,
        radial_pct: None,
        speed_pct: Some(-0.010),
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 8,
        radial_pct: None,
        speed_pct: Some(0.020),
        error_level_index: 1,
    },
    TerminalSeedSpec {
        index: 9,
        radial_pct: None,
        speed_pct: Some(-0.020),
        error_level_index: 2,
    },
    TerminalSeedSpec {
        index: 10,
        radial_pct: None,
        speed_pct: Some(0.030),
        error_level_index: 0,
    },
    TerminalSeedSpec {
        index: 11,
        radial_pct: None,
        speed_pct: Some(-0.030),
        error_level_index: 1,
    },
];

pub(super) const SIGNED_ROUTE_ARC_TRANSFER_V1_ROUTE_ANGLES: [TransferRouteAngleSpec; 11] = [
    TransferRouteAngleSpec {
        id: "r-80",
        angle_deg: -80.0,
    },
    TransferRouteAngleSpec {
        id: "r-60",
        angle_deg: -60.0,
    },
    TransferRouteAngleSpec {
        id: "r-45",
        angle_deg: -45.0,
    },
    TransferRouteAngleSpec {
        id: "r-30",
        angle_deg: -30.0,
    },
    TransferRouteAngleSpec {
        id: "r-15",
        angle_deg: -15.0,
    },
    TransferRouteAngleSpec {
        id: "r00",
        angle_deg: 0.0,
    },
    TransferRouteAngleSpec {
        id: "r+15",
        angle_deg: 15.0,
    },
    TransferRouteAngleSpec {
        id: "r+30",
        angle_deg: 30.0,
    },
    TransferRouteAngleSpec {
        id: "r+45",
        angle_deg: 45.0,
    },
    TransferRouteAngleSpec {
        id: "r+60",
        angle_deg: 60.0,
    },
    TransferRouteAngleSpec {
        id: "r+80",
        angle_deg: 80.0,
    },
];

pub(super) const SIGNED_ROUTE_ARC_TRANSFER_V1_SMOKE_ROUTE_ANGLES: [&str; 5] =
    ["r-60", "r-30", "r00", "r+30", "r+60"];

pub(super) const SIGNED_ROUTE_ARC_TRANSFER_V1_NOMINAL_RADIUS_M: f64 = 800.0;
pub(super) const SIGNED_ROUTE_ARC_TRANSFER_V1_RADIUS_TIERS: [TransferRadiusTierSpec; 3] = [
    TransferRadiusTierSpec {
        id: "short",
        radius_m: 400.0,
    },
    TransferRadiusTierSpec {
        id: "nominal",
        radius_m: SIGNED_ROUTE_ARC_TRANSFER_V1_NOMINAL_RADIUS_M,
    },
    TransferRadiusTierSpec {
        id: "long",
        radius_m: 1200.0,
    },
];

pub(super) const SIGNED_ROUTE_ARC_TRANSFER_V1_SPEC: TransferRouteFamilySpec =
    TransferRouteFamilySpec {
        route_family: "signed_route_arc_transfer_v1",
        gravity_mps2: 9.81,
        default_radius_tier: "nominal",
        radius_tiers: &SIGNED_ROUTE_ARC_TRANSFER_V1_RADIUS_TIERS,
        route_angles: &SIGNED_ROUTE_ARC_TRANSFER_V1_ROUTE_ANGLES,
        smoke_route_angles: &SIGNED_ROUTE_ARC_TRANSFER_V1_SMOKE_ROUTE_ANGLES,
    };

pub(super) const TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1: &str = "single_dogleg_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1: &str = "single_bend_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1: &str = "single_gentle_bend_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1: &str = "single_medium_bend_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1: &str = "single_sharp_bend_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1: &str = "double_bend_v1";
pub(super) const TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1: &str = "late_bend_v1";
pub(super) const TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1: &str = "legacy_v1";
pub(super) const TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1: &str = "pass_through_v1";
pub(super) const TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1: &str =
    "continuation_pass_through_v1";
pub(super) const TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1: &str =
    "sequence_pass_through_v1";
pub(super) const TRANSFER_WAYPOINT_EXPECTATION_TIER_DIAGNOSTIC: &str = "diagnostic";
pub(super) const TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC: f64 = 0.55;
pub(super) const TRANSFER_WAYPOINT_SINGLE_BEND_LATERAL_OFFSET_RATIO: f64 = 0.20;
pub(super) const TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE: f64 = 1.0e-6;
pub(super) const TRANSFER_WAYPOINT_TURN_TOLERANCE_DEG: f64 = 1.0e-3;
pub(super) const TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO: f64 = 0.75;

#[derive(Clone, Copy)]
pub(super) struct TransferWaypointBendProfileSpec {
    waypoint_id: &'static str,
    progress_frac: f64,
    lateral_offset_ratio: f64,
    capture_radius_ratio: f64,
    min_capture_radius_m: f64,
    max_capture_radius_m: f64,
    max_cross_track_factor: f64,
    min_route_angle_deg: Option<f64>,
}

#[derive(Clone, Copy)]
pub(super) struct TransferWaypointGeometryExpectation {
    progress_frac: f64,
    lateral_offset_ratio: f64,
    signed_turn_deg: f64,
}

pub(super) const SINGLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -43.9456,
    }];
pub(super) const SINGLE_GENTLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.12,
        signed_turn_deg: -27.2394,
    }];
pub(super) const SINGLE_MEDIUM_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -43.9456,
    }];
pub(super) const SINGLE_SHARP_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 1] =
    [TransferWaypointGeometryExpectation {
        progress_frac: 0.55,
        lateral_offset_ratio: 0.30,
        signed_turn_deg: -62.3005,
    }];
pub(super) const DOUBLE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 2] = [
    TransferWaypointGeometryExpectation {
        progress_frac: 0.33,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -31.2184,
    },
    TransferWaypointGeometryExpectation {
        progress_frac: 0.67,
        lateral_offset_ratio: 0.20,
        signed_turn_deg: -31.2184,
    },
];
pub(super) const LATE_BEND_GEOMETRY: [TransferWaypointGeometryExpectation; 2] = [
    TransferWaypointGeometryExpectation {
        progress_frac: 0.33,
        lateral_offset_ratio: 0.13,
        signed_turn_deg: -0.5769,
    },
    TransferWaypointGeometryExpectation {
        progress_frac: 0.67,
        lateral_offset_ratio: 0.26,
        signed_turn_deg: -59.1583,
    },
];

pub(super) const TRANSFER_SMOKE_SEEDS: [TransferSeedSpec; 3] = [
    TransferSeedSpec {
        index: 0,
        radius_pct: 0.0,
    },
    TransferSeedSpec {
        index: 1,
        radius_pct: -0.03,
    },
    TransferSeedSpec {
        index: 2,
        radius_pct: 0.03,
    },
];

pub(super) const TRANSFER_FULL_SEEDS: [TransferSeedSpec; 12] = [
    TransferSeedSpec {
        index: 0,
        radius_pct: 0.0,
    },
    TransferSeedSpec {
        index: 1,
        radius_pct: -0.015,
    },
    TransferSeedSpec {
        index: 2,
        radius_pct: 0.015,
    },
    TransferSeedSpec {
        index: 3,
        radius_pct: -0.03,
    },
    TransferSeedSpec {
        index: 4,
        radius_pct: 0.03,
    },
    TransferSeedSpec {
        index: 5,
        radius_pct: -0.045,
    },
    TransferSeedSpec {
        index: 6,
        radius_pct: 0.045,
    },
    TransferSeedSpec {
        index: 7,
        radius_pct: -0.06,
    },
    TransferSeedSpec {
        index: 8,
        radius_pct: 0.06,
    },
    TransferSeedSpec {
        index: 9,
        radius_pct: -0.075,
    },
    TransferSeedSpec {
        index: 10,
        radius_pct: 0.075,
    },
    TransferSeedSpec {
        index: 11,
        radius_pct: -0.09,
    },
];

pub(super) fn resolve_transfer_matrix_runs(
    entry: &TransferMatrixEntry,
    base_dir: &Path,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let family_spec = transfer_route_family_spec(&entry.transfer_matrix)?;
    let route_angle_specs = selected_transfer_route_angle_specs(entry, family_spec)?;
    let radius_tier_specs = selected_transfer_radius_tier_specs(entry, family_spec)?;
    let seed_specs = transfer_seed_specs(entry.seed_tier);
    let mut runs = Vec::new();

    for lane in &entry.lanes {
        let controller_spec = load_controller_spec(
            base_dir,
            lane.controller.as_str(),
            lane.controller_config.as_deref(),
        )?;
        for route_angle in &route_angle_specs {
            for radius_tier in &radius_tier_specs {
                for seed_spec in seed_specs {
                    let run_id = resolved_transfer_matrix_run_id(
                        &entry.id,
                        route_angle.id,
                        radius_tier.id,
                        seed_spec.index,
                        &lane.id,
                    );
                    let (scenario, resolved_parameters, selector) =
                        resolve_transfer_matrix_scenario(TransferMatrixScenarioRequest {
                            entry,
                            base_scenario: &base_scenario,
                            family_spec,
                            route_angle,
                            radius_tier,
                            seed_spec,
                            lane_id: &lane.id,
                            run_id: &run_id,
                        })?;
                    let descriptor = ResolvedRunDescriptor {
                        run_id,
                        entry_id: entry.id.clone(),
                        source_kind: ResolvedRunSourceKind::TransferMatrix,
                        scenario_source: entry.base_scenario.clone(),
                        resolved_scenario_id: scenario.id.clone(),
                        resolved_scenario_name: scenario.name.clone(),
                        family_id: Some(entry.id.clone()),
                        selector,
                        lane_id: lane.id.clone(),
                        resolved_seed: seed_spec.index,
                        resolved_parameters,
                        controller_id: controller_spec.id().to_owned(),
                        controller_spec: controller_spec.clone(),
                    };
                    runs.push(ResolvedBatchRun {
                        descriptor,
                        scenario,
                    });
                }
            }
        }
    }

    Ok(runs)
}

pub(super) fn selected_transfer_route_angle_specs<'a>(
    entry: &TransferMatrixEntry,
    family_spec: &'a TransferRouteFamilySpec,
) -> Result<Vec<&'a TransferRouteAngleSpec>> {
    if entry.route_angles.is_empty() {
        return Ok(match entry.seed_tier {
            TransferSeedTier::Smoke => family_spec
                .route_angles
                .iter()
                .filter(|candidate| family_spec.smoke_route_angles.contains(&candidate.id))
                .collect(),
            TransferSeedTier::Full => family_spec.route_angles.iter().collect(),
        });
    }

    entry
        .route_angles
        .iter()
        .map(|route_angle| {
            family_spec
                .route_angles
                .iter()
                .find(|candidate| candidate.id == route_angle)
                .with_context(|| {
                    format!(
                        "transfer matrix entry '{}' route_angle selector '{}' is not supported by matrix '{}'",
                        entry.id, route_angle, entry.transfer_matrix
                    )
                })
        })
        .collect()
}

pub(super) fn selected_transfer_radius_tier_specs<'a>(
    entry: &TransferMatrixEntry,
    family_spec: &'a TransferRouteFamilySpec,
) -> Result<Vec<&'a TransferRadiusTierSpec>> {
    if entry.radius_tiers.is_empty() {
        return family_spec
            .radius_tiers
            .iter()
            .find(|candidate| candidate.id == family_spec.default_radius_tier)
            .map(|candidate| vec![candidate])
            .with_context(|| {
                format!(
                    "transfer matrix '{}' default radius_tier '{}' is not supported",
                    family_spec.route_family, family_spec.default_radius_tier
                )
            });
    }

    entry
        .radius_tiers
        .iter()
        .map(|radius_tier| {
            family_spec
                .radius_tiers
                .iter()
                .find(|candidate| candidate.id == radius_tier)
                .with_context(|| {
                    format!(
                        "transfer matrix entry '{}' radius_tier selector '{}' is not supported by matrix '{}'",
                        entry.id, radius_tier, entry.transfer_matrix
                    )
                })
        })
        .collect()
}

pub(super) fn transfer_route_family_spec(name: &str) -> Result<&'static TransferRouteFamilySpec> {
    match name {
        "signed_route_arc_transfer_v1" => Ok(&SIGNED_ROUTE_ARC_TRANSFER_V1_SPEC),
        _ => bail!("unsupported transfer matrix '{}'", name),
    }
}

pub(super) fn transfer_seed_specs(seed_tier: TransferSeedTier) -> &'static [TransferSeedSpec] {
    match seed_tier {
        TransferSeedTier::Smoke => &TRANSFER_SMOKE_SEEDS,
        TransferSeedTier::Full => &TRANSFER_FULL_SEEDS,
    }
}

pub(super) fn resolved_transfer_matrix_run_id(
    entry_id: &str,
    route_angle: &str,
    radius_tier: &str,
    seed: u64,
    lane_id: &str,
) -> String {
    sanitize_token(&format!(
        "{entry_id}__{}__{radius_tier}__seed_{seed:02}__{lane_id}",
        signed_selector_token(route_angle)
    ))
}

pub(super) fn signed_selector_token(value: &str) -> String {
    value.replace('+', "pos").replace('-', "neg")
}

#[derive(Clone, Copy)]
pub(super) struct TransferMatrixScenarioRequest<'a> {
    entry: &'a TransferMatrixEntry,
    base_scenario: &'a ScenarioSpec,
    family_spec: &'a TransferRouteFamilySpec,
    route_angle: &'a TransferRouteAngleSpec,
    radius_tier: &'a TransferRadiusTierSpec,
    seed_spec: &'a TransferSeedSpec,
    lane_id: &'a str,
    run_id: &'a str,
}

pub(super) fn resolve_transfer_matrix_scenario(
    request: TransferMatrixScenarioRequest<'_>,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>, SelectorAxes)> {
    let TransferMatrixScenarioRequest {
        entry,
        base_scenario,
        family_spec,
        route_angle,
        radius_tier,
        seed_spec,
        lane_id,
        run_id,
    } = request;
    let mut scenario = base_scenario.clone();
    scenario.id = run_id.to_owned();
    scenario.name = format!(
        "{} [{} {} {} {} seed {} {}]",
        base_scenario.name,
        family_spec.route_family,
        entry.vehicle_variant,
        route_angle.id,
        radius_tier.id,
        seed_spec.index,
        lane_id
    );
    scenario.description = format!(
        "{} ({} {} {} {} {} seed {} lane {})",
        base_scenario.description,
        "transfer_matrix",
        family_spec.route_family,
        entry.vehicle_variant,
        route_angle.id,
        radius_tier.id,
        seed_spec.index,
        lane_id
    );
    scenario.seed = seed_spec.index;
    scenario.sim.max_time_s = scenario
        .sim
        .max_time_s
        .max(if entry.waypoint_profile.is_some() {
            130.0
        } else {
            90.0
        });
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed_spec.index.to_string());
    scenario
        .metadata
        .insert("mission".to_owned(), "transfer_guidance".to_owned());
    scenario.metadata.insert(
        "arrival_family".to_owned(),
        family_spec.route_family.to_owned(),
    );
    scenario.metadata.insert(
        "route_family".to_owned(),
        family_spec.route_family.to_owned(),
    );
    scenario
        .metadata
        .insert("condition_set".to_owned(), "clean".to_owned());
    scenario
        .metadata
        .insert("vehicle_variant".to_owned(), entry.vehicle_variant.clone());
    scenario.metadata.insert(
        "expectation_tier".to_owned(),
        entry.expectation_tier.clone(),
    );
    scenario
        .metadata
        .insert("arc_point".to_owned(), route_angle.id.to_owned());
    scenario
        .metadata
        .insert("velocity_band".to_owned(), radius_tier.id.to_owned());
    scenario
        .metadata
        .insert("route_angle".to_owned(), route_angle.id.to_owned());
    scenario
        .metadata
        .insert("radius_tier".to_owned(), radius_tier.id.to_owned());
    scenario
        .metadata
        .insert("lane_id".to_owned(), lane_id.to_owned());
    scenario.metadata.insert(
        "evaluation_goal".to_owned(),
        entry.evaluation_goal.as_str().to_owned(),
    );
    let resolved_waypoint_envelope = entry.waypoint_profile.as_ref().map(|_| {
        entry
            .waypoint_handoff_envelope
            .as_deref()
            .unwrap_or(TRANSFER_WAYPOINT_ENVELOPE_LEGACY_V1)
    });
    if let Some(envelope) = resolved_waypoint_envelope {
        scenario
            .metadata
            .insert("waypoint_handoff_envelope".to_owned(), envelope.to_owned());
    }

    scenario.world.gravity_mps2 = family_spec.gravity_mps2;
    let mut resolved_parameters = BTreeMap::new();
    resolved_parameters.insert("gravity_mps2".to_owned(), family_spec.gravity_mps2);
    resolved_parameters.insert("route_angle_deg".to_owned(), route_angle.angle_deg);
    let route_radius_jitter_m = radius_tier.radius_m * seed_spec.radius_pct;
    let route_radius_m = radius_tier.radius_m + route_radius_jitter_m;
    let resolved_radius_tier = TransferRadiusTierSpec {
        id: radius_tier.id,
        radius_m: route_radius_m,
    };
    resolved_parameters.insert("route_radius_nominal_m".to_owned(), radius_tier.radius_m);
    resolved_parameters.insert("route_radius_pct".to_owned(), seed_spec.radius_pct);
    resolved_parameters.insert("route_radius_jitter_m".to_owned(), route_radius_jitter_m);
    resolved_parameters.insert("route_radius_m".to_owned(), route_radius_m);
    scenario.metadata.insert(
        "resolved.route_radius_nominal_m".to_owned(),
        format!("{:.6}", radius_tier.radius_m),
    );
    scenario.metadata.insert(
        "resolved.route_radius_pct".to_owned(),
        format!("{:.6}", seed_spec.radius_pct),
    );
    scenario.metadata.insert(
        "resolved.route_radius_jitter_m".to_owned(),
        format!("{route_radius_jitter_m:.6}"),
    );
    scenario.metadata.insert(
        "resolved.seed_variation".to_owned(),
        if seed_spec.radius_pct.abs() > f64::EPSILON {
            "route_radius".to_owned()
        } else {
            "none".to_owned()
        },
    );

    for adjustment in &entry.adjustments {
        apply_numeric_adjustment(&mut scenario, adjustment)?;
        resolved_parameters.insert(adjustment.id.clone(), adjustment.value);
        scenario.metadata.insert(
            format!("resolved.{}", adjustment.id),
            format!("{:.6}", adjustment.value),
        );
    }

    let (source_pad, target_pad) = configure_transfer_route_geometry(
        &mut scenario,
        route_angle,
        &resolved_radius_tier,
        entry.waypoint_profile.as_deref(),
        resolved_waypoint_envelope,
    )?;
    resolved_parameters.insert("source_x_m".to_owned(), source_pad.center_x_m);
    resolved_parameters.insert("source_y_m".to_owned(), source_pad.surface_y_m);
    resolved_parameters.insert("target_x_m".to_owned(), target_pad.center_x_m);
    resolved_parameters.insert("target_y_m".to_owned(), target_pad.surface_y_m);
    resolved_parameters.insert(
        "route_dx_m".to_owned(),
        target_pad.center_x_m - source_pad.center_x_m,
    );
    resolved_parameters.insert(
        "route_dy_m".to_owned(),
        target_pad.surface_y_m - source_pad.surface_y_m,
    );
    resolved_parameters.insert("start_x_m".to_owned(), scenario.initial_state.position_m.x);
    resolved_parameters.insert("start_y_m".to_owned(), scenario.initial_state.position_m.y);
    if entry.evaluation_goal == TransferMatrixEvaluationGoal::WaypointHandoff {
        scenario.mission.goal = EvaluationGoal::WaypointHandoff {
            target_pad_id: target_pad.id.clone(),
            waypoint_index: 0,
        };
        resolved_parameters.insert("waypoint_handoff_index".to_owned(), 0.0);
    } else if entry.evaluation_goal == TransferMatrixEvaluationGoal::WaypointSequence {
        scenario.mission.goal = EvaluationGoal::WaypointSequence {
            target_pad_id: target_pad.id.clone(),
        };
    }
    if let Some(route) = scenario.mission.transfer_route.as_ref() {
        insert_waypoint_geometry_resolved_parameters(
            &mut resolved_parameters,
            Vec2::new(source_pad.center_x_m, source_pad.surface_y_m),
            Vec2::new(target_pad.center_x_m, target_pad.surface_y_m),
            &scenario.world.terrain,
            &route.waypoints,
            route.route_radius_m,
            &scenario.vehicle,
        )?;
        for (index, waypoint) in route.waypoints.iter().enumerate() {
            let prefix = format!("waypoint_{index}");
            resolved_parameters.insert(format!("{prefix}_x_m"), waypoint.position_m.x);
            resolved_parameters.insert(format!("{prefix}_y_m"), waypoint.position_m.y);
            resolved_parameters.insert(
                format!("{prefix}_capture_radius_m"),
                waypoint.capture_radius_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_max_cross_track_m"),
                waypoint.max_cross_track_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_max_outbound_heading_error_rad"),
                waypoint.max_outbound_heading_error_rad,
            );
            resolved_parameters.insert(
                format!("{prefix}_min_outbound_progress_mps"),
                waypoint.min_outbound_progress_mps,
            );
            if let Some(max_cross_speed_mps) = waypoint.max_outbound_cross_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_max_outbound_cross_speed_mps"),
                    max_cross_speed_mps,
                );
            }
            resolved_parameters.insert(format!("{prefix}_min_speed_mps"), waypoint.min_speed_mps);
            resolved_parameters.insert(format!("{prefix}_max_speed_mps"), waypoint.max_speed_mps);
            if let Some(min_vertical_speed_mps) = waypoint.min_vertical_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_min_vertical_speed_mps"),
                    min_vertical_speed_mps,
                );
            }
            if let Some(max_vertical_speed_mps) = waypoint.max_vertical_speed_mps {
                resolved_parameters.insert(
                    format!("{prefix}_max_vertical_speed_mps"),
                    max_vertical_speed_mps,
                );
            }
        }
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved transfer matrix scenario '{}' {} seed {} failed validation",
                entry.id, route_angle.id, seed_spec.index
            )
        })?;

    let selector = SelectorAxes {
        mission: "transfer_guidance".to_owned(),
        arrival_family: family_spec.route_family.to_owned(),
        condition_set: "clean".to_owned(),
        vehicle_variant: entry.vehicle_variant.clone(),
        arc_point: route_angle.id.to_owned(),
        velocity_band: radius_tier.id.to_owned(),
        route_family: family_spec.route_family.to_owned(),
        route_angle: route_angle.id.to_owned(),
        radius_tier: radius_tier.id.to_owned(),
        waypoint_profile: entry
            .waypoint_profile
            .clone()
            .unwrap_or_else(default_selector_value),
        waypoint_handoff_envelope: resolved_waypoint_envelope
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_selector_value),
        expectation_tier: Some(entry.expectation_tier.clone()),
    };

    Ok((scenario, resolved_parameters, selector))
}

pub(super) fn configure_transfer_route_geometry(
    scenario: &mut ScenarioSpec,
    route_angle: &TransferRouteAngleSpec,
    radius_tier: &TransferRadiusTierSpec,
    waypoint_profile: Option<&str>,
    waypoint_handoff_envelope: Option<&str>,
) -> Result<(LandingPadSpec, LandingPadSpec)> {
    let target_pad_id = scenario.mission.goal.target_pad_id().to_owned();
    let base_target_pad = scenario
        .world
        .landing_pad(&target_pad_id)
        .cloned()
        .ok_or_else(|| {
            anyhow!("transfer matrix base scenario is missing target pad '{target_pad_id}'")
        })?;
    let route_angle_rad = route_angle.angle_deg.to_radians();
    let dx_m = radius_tier.radius_m * route_angle_rad.cos();
    let dy_m = radius_tier.radius_m * route_angle_rad.sin();
    let target_pad = LandingPadSpec {
        id: target_pad_id,
        center_x_m: 0.0,
        surface_y_m: 0.0,
        width_m: base_target_pad.width_m,
    };
    let source_pad = LandingPadSpec {
        id: "pad_source".to_owned(),
        center_x_m: -dx_m,
        surface_y_m: -dy_m,
        width_m: base_target_pad.width_m,
    };
    let terrain_points = transfer_route_terrain_points(&source_pad, &target_pad)?;

    scenario.world.terrain = TerrainDefinition::Heightfield {
        points_m: terrain_points,
    };
    scenario.world.landing_pads = vec![source_pad.clone(), target_pad.clone()];
    scenario.initial_state.position_m = Vec2::new(
        source_pad.center_x_m,
        source_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m,
    );
    scenario.initial_state.velocity_mps = Vec2::new(0.0, 0.0);
    scenario.initial_state.attitude_rad = 0.0;
    scenario.initial_state.angular_rate_radps = 0.0;
    let mut waypoints = transfer_route_waypoints_for_profile(
        waypoint_profile,
        &source_pad,
        &target_pad,
        route_angle,
        radius_tier,
    )?;
    apply_transfer_waypoint_envelope(&mut waypoints, waypoint_handoff_envelope, radius_tier.id)?;
    if let Some(profile) =
        waypoint_profile.filter(|profile| *profile != TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1)
    {
        validate_transfer_waypoint_geometry(
            profile,
            &source_pad,
            &target_pad,
            &scenario.world.terrain,
            &waypoints,
            radius_tier.radius_m,
            scenario.vehicle.geometry.touchdown_base_offset_m,
        )?;
        validate_transfer_waypoint_continuation(
            profile,
            &target_pad,
            &waypoints,
            &scenario.vehicle,
        )?;
        if profile == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 {
            validate_transfer_waypoint_turn_authority(
                profile,
                &source_pad,
                &target_pad,
                &waypoints,
                &scenario.vehicle,
            )?;
        }
    }
    scenario.mission.transfer_route = Some(TransferRouteSpec {
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        route_angle_deg: route_angle.angle_deg,
        route_radius_m: radius_tier.radius_m,
        waypoints,
    });

    scenario
        .metadata
        .insert("resolved.source_pad_id".to_owned(), source_pad.id.clone());
    scenario
        .metadata
        .insert("resolved.target_pad_id".to_owned(), target_pad.id.clone());
    scenario.metadata.insert(
        "resolved.route_angle_deg".to_owned(),
        format!("{:.6}", route_angle.angle_deg),
    );
    scenario.metadata.insert(
        "resolved.route_radius_m".to_owned(),
        format!("{:.6}", radius_tier.radius_m),
    );
    scenario.metadata.insert(
        "route_mode".to_owned(),
        waypoint_profile.unwrap_or("direct").to_owned(),
    );
    scenario.metadata.insert(
        "waypoint_profile".to_owned(),
        waypoint_profile.unwrap_or("direct").to_owned(),
    );
    if let Some(envelope) = waypoint_handoff_envelope {
        scenario
            .metadata
            .insert("waypoint_handoff_envelope".to_owned(), envelope.to_owned());
    }
    if let Some(route) = scenario.mission.transfer_route.as_ref() {
        for (index, waypoint) in route.waypoints.iter().enumerate() {
            scenario
                .metadata
                .insert(format!("resolved.waypoint_{index}.id"), waypoint.id.clone());
            scenario.metadata.insert(
                format!("resolved.waypoint_{index}.x_m"),
                format!("{:.6}", waypoint.position_m.x),
            );
            scenario.metadata.insert(
                format!("resolved.waypoint_{index}.y_m"),
                format!("{:.6}", waypoint.position_m.y),
            );
        }
    }

    Ok((source_pad, target_pad))
}

pub(super) fn transfer_route_waypoints_for_profile(
    waypoint_profile: Option<&str>,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    route_angle: &TransferRouteAngleSpec,
    radius_tier: &TransferRadiusTierSpec,
) -> Result<Vec<TransferWaypointSpec>> {
    let Some(profile) = waypoint_profile else {
        return Ok(Vec::new());
    };
    validate_transfer_waypoint_profile("resolved transfer matrix", profile)?;
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_DOGLEG_V1 => {
            if route_angle.angle_deg < 70.0 {
                bail!(
                    "waypoint profile '{}' requires a steep uphill route angle, got '{}'",
                    profile,
                    route_angle.id
                );
            }
            let route_dx_m = target_pad.center_x_m - source_pad.center_x_m;
            let direction = if route_dx_m >= 0.0 { 1.0 } else { -1.0 };
            let radius_m = radius_tier.radius_m;
            let capture_radius_m = (radius_m * 0.08).clamp(35.0, 95.0);
            Ok(vec![TransferWaypointSpec {
                id: "wp_dogleg_01".to_owned(),
                position_m: Vec2::new(
                    source_pad.center_x_m - (direction * radius_m * 0.70),
                    target_pad.surface_y_m + (radius_m * 0.45),
                ),
                handoff_tangent_unit: None,
                capture_radius_m,
                max_cross_track_m: capture_radius_m * 1.25,
                max_outbound_heading_error_rad: 0.85,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 10.0,
                max_speed_mps: 130.0,
                min_vertical_speed_mps: Some(-80.0),
                max_vertical_speed_mps: Some(65.0),
            }])
        }
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => {
            transfer_route_sequence_waypoints(profile, source_pad, target_pad, radius_tier)
        }
        _ => {
            let profile_spec = transfer_waypoint_bend_profile_spec(profile)
                .expect("validated bend waypoint profile should have geometry");
            if profile_spec
                .min_route_angle_deg
                .is_some_and(|minimum| route_angle.angle_deg < minimum)
            {
                bail!(
                    "waypoint profile '{}' requires a steep uphill route angle, got '{}'",
                    profile,
                    route_angle.id
                );
            }
            let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
            let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
            let route_m = target_m - source_m;
            let route_unit_m = waypoint_normalized(route_m).ok_or_else(|| {
                anyhow!("waypoint profile '{profile}' cannot resolve a zero-length route")
            })?;
            let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
            let source_side_normal_m =
                Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);
            let radius_m = radius_tier.radius_m;
            let position_m = source_m
                + (route_m * profile_spec.progress_frac)
                + (source_side_normal_m * (radius_m * profile_spec.lateral_offset_ratio));
            let capture_radius_m = (radius_m * profile_spec.capture_radius_ratio).clamp(
                profile_spec.min_capture_radius_m,
                profile_spec.max_capture_radius_m,
            );
            Ok(vec![TransferWaypointSpec {
                id: profile_spec.waypoint_id.to_owned(),
                position_m,
                handoff_tangent_unit: None,
                capture_radius_m,
                max_cross_track_m: capture_radius_m * profile_spec.max_cross_track_factor,
                max_outbound_heading_error_rad: 0.85,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: None,
                min_speed_mps: 10.0,
                max_speed_mps: 130.0,
                min_vertical_speed_mps: Some(-80.0),
                max_vertical_speed_mps: Some(65.0),
            }])
        }
    }
}

pub(super) fn transfer_route_sequence_waypoints(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    radius_tier: &TransferRadiusTierSpec,
) -> Result<Vec<TransferWaypointSpec>> {
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    let route_m = target_m - source_m;
    let route_unit_m = waypoint_normalized(route_m).ok_or_else(|| {
        anyhow!("waypoint profile '{profile}' cannot resolve a zero-length route")
    })?;
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);
    let radius_m = radius_tier.radius_m;
    let maintained = profile == TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1;
    let capture_radius_m = if maintained {
        (radius_m * 0.08).min(95.0)
    } else {
        (radius_m * 0.08).clamp(35.0, 95.0)
    };
    let speed_scale = if maintained {
        (radius_m / SIGNED_ROUTE_ARC_TRANSFER_V1_NOMINAL_RADIUS_M).sqrt()
    } else {
        1.0
    };
    let node_specs = match profile {
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 => [
            ("wp_double_bend_01", 0.33, 0.20, 55.0),
            ("wp_double_bend_02", 0.67, 0.20, 65.0),
        ],
        TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => [
            ("wp_late_bend_01", 0.33, 0.13, 45.0),
            ("wp_late_bend_02", 0.67, 0.26, 65.0),
        ],
        _ => unreachable!("validated sequence waypoint profile"),
    };
    let mut waypoints = node_specs
        .into_iter()
        .map(
            |(waypoint_id, progress_frac, lateral_offset_ratio, max_speed_mps)| {
                let position_m = source_m
                    + (route_m * progress_frac)
                    + (source_side_normal_m * (radius_m * lateral_offset_ratio));
                TransferWaypointSpec {
                    id: waypoint_id.to_owned(),
                    position_m,
                    handoff_tangent_unit: None,
                    capture_radius_m,
                    max_cross_track_m: capture_radius_m * 1.25,
                    max_outbound_heading_error_rad: 0.35,
                    min_outbound_progress_mps: 8.0,
                    max_outbound_cross_speed_mps: Some(20.0),
                    min_speed_mps: 10.0,
                    max_speed_mps: max_speed_mps * speed_scale,
                    min_vertical_speed_mps: None,
                    max_vertical_speed_mps: None,
                }
            },
        )
        .collect::<Vec<_>>();
    apply_transfer_waypoint_handoff_tangents(profile, source_m, target_m, &mut waypoints)?;
    Ok(waypoints)
}

pub(super) fn apply_transfer_waypoint_handoff_tangents(
    profile: &str,
    source_m: Vec2,
    target_m: Vec2,
    waypoints: &mut [TransferWaypointSpec],
) -> Result<()> {
    for index in 0..waypoints.len() {
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map_or(target_m, |next| next.position_m);
        let inbound_unit = waypoint_normalized(waypoints[index].position_m - anchor_m)
            .ok_or_else(|| anyhow!("waypoint profile '{profile}' has a zero-length inbound leg"))?;
        let outbound_unit = waypoint_normalized(next_target_m - waypoints[index].position_m)
            .ok_or_else(|| {
                anyhow!("waypoint profile '{profile}' has a zero-length outbound leg")
            })?;
        let tangent = waypoint_normalized(inbound_unit + outbound_unit).ok_or_else(|| {
            anyhow!("waypoint profile '{profile}' cannot bisect opposing route legs")
        })?;
        waypoints[index].handoff_tangent_unit = Some(tangent);
    }
    Ok(())
}

pub(super) fn validate_transfer_waypoint_geometry(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    terrain: &TerrainDefinition,
    waypoints: &[TransferWaypointSpec],
    route_radius_m: f64,
    touchdown_base_offset_m: f64,
) -> Result<()> {
    let expectations = transfer_waypoint_geometry_expectations(profile)
        .expect("maintained waypoint profile must define geometry expectations");
    if waypoints.len() != expectations.len() {
        bail!(
            "waypoint profile '{profile}' must resolve exactly {} waypoints, got {}",
            expectations.len(),
            waypoints.len()
        );
    }
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    let route_m = target_m - source_m;
    let route_length_m = route_m.length();
    let route_unit_m = waypoint_normalized(route_m)
        .ok_or_else(|| anyhow!("waypoint profile '{profile}' resolved a zero-length route"))?;
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);

    let mut previous_progress = 0.0;
    for (index, (waypoint, expected)) in waypoints.iter().zip(expectations).enumerate() {
        let from_source_m = waypoint.position_m - source_m;
        let progress = waypoint_dot(from_source_m, route_unit_m) / route_length_m;
        let signed_offset_ratio =
            waypoint_dot(from_source_m, source_side_normal_m) / route_radius_m;
        if !(progress > previous_progress && progress < 1.0) {
            bail!(
                "waypoint profile '{profile}' waypoint {index} must preserve strict route order, got progress {progress:.6}"
            );
        }
        if signed_offset_ratio <= 0.0 {
            bail!(
                "waypoint profile '{profile}' waypoint {index} must remain on the positive source-side normal, got {signed_offset_ratio:.6}"
            );
        }
        if (progress - expected.progress_frac).abs() > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE
            || (signed_offset_ratio - expected.lateral_offset_ratio).abs()
                > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} route geometry ({progress:.6}, {signed_offset_ratio:.6}) does not match ({:.6}, {:.6})",
                expected.progress_frac,
                expected.lateral_offset_ratio
            );
        }
        previous_progress = progress;
    }

    for (index, pair) in waypoints.windows(2).enumerate() {
        let waypoint_separation_m = (pair[1].position_m - pair[0].position_m).length();
        if waypoint_separation_m <= pair[0].capture_radius_m + pair[1].capture_radius_m {
            bail!(
                "waypoint profile '{profile}' waypoints {index} and {} have overlapping capture regions ({waypoint_separation_m:.3}m)",
                index + 1
            );
        }
    }
    for waypoint in waypoints {
        let terrain_clearance_m =
            waypoint.position_m.y - terrain.sample_height(waypoint.position_m.x);
        let required_clearance_m =
            waypoint.capture_radius_m.max(waypoint.max_cross_track_m) + touchdown_base_offset_m;
        if terrain_clearance_m <= required_clearance_m {
            bail!(
                "waypoint profile '{profile}' waypoint '{}' terrain clearance {:.3}m must exceed {:.3}m",
                waypoint.id,
                terrain_clearance_m,
                required_clearance_m
            );
        }
    }

    let mut geometry_nodes = Vec::with_capacity(waypoints.len() + 2);
    geometry_nodes.push(source_m);
    geometry_nodes.extend(waypoints.iter().map(|waypoint| waypoint.position_m));
    geometry_nodes.push(target_m);
    let mut previous_heading_rad: Option<f64> = None;
    for (segment_index, segment) in geometry_nodes.windows(2).enumerate() {
        let segment_m = segment[1] - segment[0];
        let route_progress_m = waypoint_dot(segment_m, route_unit_m);
        if route_progress_m <= 0.0 {
            bail!(
                "waypoint profile '{profile}' segment {segment_index} reverses route progress ({route_progress_m:.6}m)"
            );
        }
        let heading_rad =
            waypoint_cross(route_unit_m, segment_m).atan2(waypoint_dot(route_unit_m, segment_m));
        if previous_heading_rad
            .is_some_and(|previous| heading_rad >= previous - TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE)
        {
            bail!(
                "waypoint profile '{profile}' segment {segment_index} route-relative heading {:.6}deg must decrease from {:.6}deg",
                heading_rad.to_degrees(),
                previous_heading_rad.unwrap().to_degrees()
            );
        }
        previous_heading_rad = Some(heading_rad);
    }

    for (index, expected) in expectations.iter().enumerate() {
        let inbound_m = geometry_nodes[index + 1] - geometry_nodes[index];
        let outbound_m = geometry_nodes[index + 2] - geometry_nodes[index + 1];
        let signed_turn_deg = waypoint_cross(inbound_m, outbound_m)
            .atan2(waypoint_dot(inbound_m, outbound_m))
            .to_degrees();
        if (signed_turn_deg - expected.signed_turn_deg).abs() > TRANSFER_WAYPOINT_TURN_TOLERANCE_DEG
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} signed turn {signed_turn_deg:.6}deg must equal {:.6}deg",
                expected.signed_turn_deg
            );
        }
        if matches!(
            profile,
            TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 | TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1
        ) {
            let expected_tangent = waypoint_normalized(
                waypoint_normalized(inbound_m).unwrap() + waypoint_normalized(outbound_m).unwrap(),
            )
            .expect("validated sequence legs cannot oppose");
            let tangent = waypoints[index]
                .handoff_tangent_unit
                .ok_or_else(|| anyhow!("waypoint profile '{profile}' waypoint {index} requires an explicit handoff tangent"))?;
            if (tangent - expected_tangent).length() > TRANSFER_WAYPOINT_GEOMETRY_TOLERANCE {
                bail!(
                    "waypoint profile '{profile}' waypoint {index} handoff tangent must bisect its route legs"
                );
            }
        }
    }

    // Endpoint legs are dynamically shaped launch/landing trajectories, not straight
    // route segments. Only an explicit multi-waypoint centerline must clear terrain.
    for segment in waypoints.windows(2) {
        for sample_index in 1..24 {
            let t = sample_index as f64 / 24.0;
            let point_m =
                segment[0].position_m + ((segment[1].position_m - segment[0].position_m) * t);
            if point_m.y + 1.0e-6 < terrain.sample_height(point_m.x) + touchdown_base_offset_m {
                bail!(
                    "waypoint profile '{profile}' centerline violates terrain clearance at x={:.3}m",
                    point_m.x
                );
            }
        }
    }
    Ok(())
}

pub(super) fn transfer_waypoint_geometry_expectations(
    profile: &str,
) -> Option<&'static [TransferWaypointGeometryExpectation]> {
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1 => Some(&SINGLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1 => Some(&SINGLE_GENTLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1 => Some(&SINGLE_MEDIUM_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1 => Some(&SINGLE_SHARP_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_DOUBLE_BEND_V1 => Some(&DOUBLE_BEND_GEOMETRY),
        TRANSFER_WAYPOINT_PROFILE_LATE_BEND_V1 => Some(&LATE_BEND_GEOMETRY),
        _ => None,
    }
}

pub(super) fn transfer_waypoint_bend_profile_spec(
    profile: &str,
) -> Option<TransferWaypointBendProfileSpec> {
    let balanced = |waypoint_id, lateral_offset_ratio| TransferWaypointBendProfileSpec {
        waypoint_id,
        progress_frac: TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC,
        lateral_offset_ratio,
        capture_radius_ratio: 0.08,
        min_capture_radius_m: 0.0,
        max_capture_radius_m: 95.0,
        max_cross_track_factor: 1.25,
        min_route_angle_deg: None,
    };
    match profile {
        TRANSFER_WAYPOINT_PROFILE_SINGLE_BEND_V1 => Some(TransferWaypointBendProfileSpec {
            waypoint_id: "wp_bend_01",
            progress_frac: TRANSFER_WAYPOINT_SINGLE_BEND_PROGRESS_FRAC,
            lateral_offset_ratio: TRANSFER_WAYPOINT_SINGLE_BEND_LATERAL_OFFSET_RATIO,
            capture_radius_ratio: 0.10,
            min_capture_radius_m: 40.0,
            max_capture_radius_m: 100.0,
            max_cross_track_factor: 1.75,
            min_route_angle_deg: Some(70.0),
        }),
        TRANSFER_WAYPOINT_PROFILE_SINGLE_GENTLE_BEND_V1 => {
            Some(balanced("wp_gentle_bend_01", 0.12))
        }
        TRANSFER_WAYPOINT_PROFILE_SINGLE_MEDIUM_BEND_V1 => {
            Some(balanced("wp_medium_bend_01", 0.20))
        }
        TRANSFER_WAYPOINT_PROFILE_SINGLE_SHARP_BEND_V1 => Some(balanced("wp_sharp_bend_01", 0.30)),
        _ => None,
    }
}

pub(super) fn apply_transfer_waypoint_envelope(
    waypoints: &mut [TransferWaypointSpec],
    envelope: Option<&str>,
    radius_tier: &str,
) -> Result<()> {
    if !matches!(
        envelope,
        Some(
            TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1
                | TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1
                | TRANSFER_WAYPOINT_ENVELOPE_SEQUENCE_PASS_THROUGH_V1
        )
    ) {
        return Ok(());
    }
    let continuation_speed_cap_mps =
        if envelope == Some(TRANSFER_WAYPOINT_ENVELOPE_CONTINUATION_PASS_THROUGH_V1) {
            Some(match radius_tier {
                // Covers the full pack's -9% radius seed at full initial mass.
                "short" => 52.5,
                "nominal" => 65.0,
                "long" => 75.0,
                _ => bail!("unsupported continuation waypoint radius tier '{radius_tier}'"),
            })
        } else {
            None
        };
    for waypoint in waypoints {
        waypoint.max_outbound_heading_error_rad = 0.35;
        waypoint.min_outbound_progress_mps = 8.0;
        waypoint.max_outbound_cross_speed_mps = Some(20.0);
        waypoint.min_speed_mps = 10.0;
        if envelope == Some(TRANSFER_WAYPOINT_ENVELOPE_PASS_THROUGH_V1) {
            waypoint.max_speed_mps = 130.0;
        } else if let Some(max_speed_mps) = continuation_speed_cap_mps {
            waypoint.max_speed_mps = max_speed_mps;
        }
        waypoint.min_vertical_speed_mps = None;
        waypoint.max_vertical_speed_mps = None;
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct TransferWaypointContinuationMetrics {
    available_distance_m: f64,
    optimistic_stop_distance_m: f64,
    stop_ratio: f64,
    max_acceleration_mps2: f64,
}

pub(super) fn transfer_waypoint_continuation_metrics(
    waypoint: &TransferWaypointSpec,
    outbound_target_m: Vec2,
    vehicle: &VehicleSpec,
) -> Result<TransferWaypointContinuationMetrics> {
    let available_distance_m =
        (outbound_target_m - waypoint.position_m).length() - waypoint.capture_radius_m;
    if available_distance_m <= 0.0 {
        bail!(
            "waypoint '{}' has no continuation distance beyond its capture region",
            waypoint.id
        );
    }
    let initial_mass_kg = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
    if initial_mass_kg <= 0.0 || vehicle.max_thrust_n <= 0.0 {
        bail!("waypoint continuation requires positive initial mass and maximum thrust");
    }
    let max_acceleration_mps2 = vehicle.max_thrust_n / initial_mass_kg;
    let optimistic_stop_distance_m = waypoint.max_speed_mps.powi(2) / (2.0 * max_acceleration_mps2);
    Ok(TransferWaypointContinuationMetrics {
        available_distance_m,
        optimistic_stop_distance_m,
        stop_ratio: optimistic_stop_distance_m / available_distance_m,
        max_acceleration_mps2,
    })
}

pub(super) fn validate_transfer_waypoint_continuation(
    profile: &str,
    target_pad: &LandingPadSpec,
    waypoints: &[TransferWaypointSpec],
    vehicle: &VehicleSpec,
) -> Result<()> {
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    for (index, waypoint) in waypoints.iter().enumerate() {
        let outbound_target_m = waypoints
            .get(index + 1)
            .map(|next| next.position_m)
            .unwrap_or(target_m);
        let metrics = transfer_waypoint_continuation_metrics(waypoint, outbound_target_m, vehicle)?;
        if metrics.stop_ratio
            > TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO + REGRESSION_POLICY_EPSILON
        {
            bail!(
                "waypoint profile '{profile}' waypoint {index} continuation stop ratio {:.6} exceeds 0.750000",
                metrics.stop_ratio
            );
        }
    }
    Ok(())
}

pub(super) fn transfer_waypoint_turn_authority_ratio(
    waypoint: &TransferWaypointSpec,
    anchor_m: Vec2,
    next_target_m: Vec2,
    vehicle: &VehicleSpec,
) -> Result<f64> {
    let tangent = waypoint.handoff_tangent_unit.ok_or_else(|| {
        anyhow!(
            "waypoint '{}' requires an explicit handoff tangent",
            waypoint.id
        )
    })?;
    let inbound_m = waypoint.position_m - anchor_m;
    let outbound_m = next_target_m - waypoint.position_m;
    let inbound_unit = waypoint_normalized(inbound_m)
        .ok_or_else(|| anyhow!("waypoint '{}' has a zero-length inbound leg", waypoint.id))?;
    let outbound_unit = waypoint_normalized(outbound_m)
        .ok_or_else(|| anyhow!("waypoint '{}' has a zero-length outbound leg", waypoint.id))?;
    let initial_mass_kg = vehicle.dry_mass_kg + vehicle.initial_fuel_kg;
    let max_acceleration_mps2 = vehicle.max_thrust_n / initial_mass_kg.max(1.0);
    let side_ratio = |leg_m: Vec2, leg_unit: Vec2| {
        let available_distance_m = (leg_m.length() - waypoint.capture_radius_m).max(1.0);
        let deflection_rad = waypoint_dot(leg_unit, tangent).clamp(-1.0, 1.0).acos();
        2.0 * waypoint.max_speed_mps.powi(2) * (deflection_rad * 0.5).sin()
            / (max_acceleration_mps2 * available_distance_m)
    };
    Ok(side_ratio(inbound_m, inbound_unit).max(side_ratio(outbound_m, outbound_unit)))
}

pub(super) fn validate_transfer_waypoint_turn_authority(
    profile: &str,
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
    waypoints: &[TransferWaypointSpec],
    vehicle: &VehicleSpec,
) -> Result<()> {
    let source_m = Vec2::new(source_pad.center_x_m, source_pad.surface_y_m);
    let target_m = Vec2::new(target_pad.center_x_m, target_pad.surface_y_m);
    for (index, waypoint) in waypoints.iter().enumerate() {
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map_or(target_m, |next| next.position_m);
        let ratio =
            transfer_waypoint_turn_authority_ratio(waypoint, anchor_m, next_target_m, vehicle)?;
        if ratio > TRANSFER_WAYPOINT_CONTINUATION_MAX_STOP_RATIO + REGRESSION_POLICY_EPSILON {
            bail!(
                "waypoint profile '{profile}' waypoint {index} optimistic turn-authority ratio {ratio:.6} exceeds 0.750000"
            );
        }
    }
    Ok(())
}

pub(super) fn insert_waypoint_geometry_resolved_parameters(
    resolved_parameters: &mut BTreeMap<String, f64>,
    source_m: Vec2,
    target_m: Vec2,
    terrain: &TerrainDefinition,
    waypoints: &[TransferWaypointSpec],
    route_radius_m: f64,
    vehicle: &VehicleSpec,
) -> Result<()> {
    let route_m = target_m - source_m;
    let Some(route_unit_m) = waypoint_normalized(route_m) else {
        return Ok(());
    };
    let route_length_m = route_m.length();
    if route_length_m <= 1.0e-9 {
        return Ok(());
    }
    let direction = if route_m.x >= 0.0 { 1.0 } else { -1.0 };
    let source_side_normal_m = Vec2::new(-route_unit_m.y * direction, route_unit_m.x * direction);

    for (index, waypoint) in waypoints.iter().enumerate() {
        let prefix = format!("waypoint_{index}");
        let terrain_y_m = terrain.sample_height(waypoint.position_m.x);
        resolved_parameters.insert(format!("{prefix}_terrain_y_m"), terrain_y_m);
        resolved_parameters.insert(
            format!("{prefix}_terrain_clearance_m"),
            waypoint.position_m.y - terrain_y_m,
        );
        let anchor_m = if index == 0 {
            source_m
        } else {
            waypoints[index - 1].position_m
        };
        let next_target_m = waypoints
            .get(index + 1)
            .map(|next| next.position_m)
            .unwrap_or(target_m);
        let inbound_m = waypoint.position_m - anchor_m;
        let outbound_m = next_target_m - waypoint.position_m;
        let inbound_length_m = inbound_m.length();
        let outbound_length_m = outbound_m.length();
        resolved_parameters.insert(format!("{prefix}_inbound_leg_length_m"), inbound_length_m);
        resolved_parameters.insert(format!("{prefix}_outbound_leg_length_m"), outbound_length_m);
        if let Some(tangent) = waypoint.handoff_tangent_unit {
            let inbound_unit = waypoint_normalized(inbound_m).expect("resolved inbound leg");
            let outbound_unit = waypoint_normalized(outbound_m).expect("resolved outbound leg");
            resolved_parameters.insert(format!("{prefix}_handoff_tangent_x"), tangent.x);
            resolved_parameters.insert(format!("{prefix}_handoff_tangent_y"), tangent.y);
            resolved_parameters.insert(
                format!("{prefix}_handoff_tangent_heading_deg"),
                tangent.y.atan2(tangent.x).to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_inbound_tangent_angle_deg"),
                waypoint_dot(inbound_unit, tangent)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_tangent_outbound_angle_deg"),
                waypoint_dot(tangent, outbound_unit)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_turn_authority_ratio"),
                transfer_waypoint_turn_authority_ratio(waypoint, anchor_m, next_target_m, vehicle)?,
            );
        }
        let metrics = transfer_waypoint_continuation_metrics(waypoint, next_target_m, vehicle)?;
        resolved_parameters.insert(
            format!("{prefix}_continuation_available_distance_m"),
            metrics.available_distance_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_optimistic_stop_distance_m"),
            metrics.optimistic_stop_distance_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_stop_ratio"),
            metrics.stop_ratio,
        );
        resolved_parameters.insert(
            format!("{prefix}_continuation_max_acceleration_mps2"),
            metrics.max_acceleration_mps2,
        );
        if inbound_length_m > 1.0e-9 && outbound_length_m > 1.0e-9 {
            let turn_angle_cos =
                waypoint_dot(inbound_m, outbound_m) / (inbound_length_m * outbound_length_m);
            resolved_parameters.insert(
                format!("{prefix}_turn_angle_deg"),
                turn_angle_cos.clamp(-1.0, 1.0).acos().to_degrees(),
            );
            resolved_parameters.insert(
                format!("{prefix}_signed_turn_angle_deg"),
                waypoint_cross(inbound_m, outbound_m)
                    .atan2(waypoint_dot(inbound_m, outbound_m))
                    .to_degrees(),
            );
        }

        let from_source_m = waypoint.position_m - source_m;
        let profile_progress_frac = waypoint_dot(from_source_m, route_unit_m) / route_length_m;
        let profile_lateral_offset_m = waypoint_cross(from_source_m, route_unit_m).abs();
        let route_signed_offset_m = waypoint_dot(from_source_m, source_side_normal_m);
        resolved_parameters.insert(
            format!("{prefix}_profile_progress_frac"),
            profile_progress_frac,
        );
        resolved_parameters.insert(
            format!("{prefix}_profile_lateral_offset_m"),
            profile_lateral_offset_m,
        );
        resolved_parameters.insert(
            format!("{prefix}_route_signed_offset_m"),
            route_signed_offset_m,
        );
        if route_radius_m > 1.0e-9 {
            resolved_parameters.insert(
                format!("{prefix}_profile_lateral_offset_ratio"),
                profile_lateral_offset_m / route_radius_m,
            );
            resolved_parameters.insert(
                format!("{prefix}_route_signed_offset_ratio"),
                route_signed_offset_m / route_radius_m,
            );
        }
    }
    Ok(())
}

pub(super) fn transfer_route_terrain_points(
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
) -> Result<Vec<Vec2>> {
    let source_left_m = source_pad.center_x_m - source_pad.half_width_m();
    let source_right_m = source_pad.center_x_m + source_pad.half_width_m();
    let target_left_m = target_pad.center_x_m - target_pad.half_width_m();
    let target_right_m = target_pad.center_x_m + target_pad.half_width_m();
    if source_right_m >= target_left_m {
        bail!(
            "transfer route geometry overlaps source and target pads: source_right={source_right_m:.3}, target_left={target_left_m:.3}"
        );
    }
    let route_span_m = target_pad.center_x_m - source_pad.center_x_m;
    let margin_m = (route_span_m.abs() * 0.15).max(160.0);
    let points = vec![
        Vec2::new(source_pad.center_x_m - margin_m, source_pad.surface_y_m),
        Vec2::new(source_left_m, source_pad.surface_y_m),
        Vec2::new(source_right_m, source_pad.surface_y_m),
        Vec2::new(target_left_m, target_pad.surface_y_m),
        Vec2::new(target_right_m, target_pad.surface_y_m),
        Vec2::new(target_pad.center_x_m + margin_m, target_pad.surface_y_m),
    ];
    validate_transfer_route_terrain(&points, source_pad, target_pad)?;
    Ok(points)
}

pub(super) fn validate_transfer_route_terrain(
    points: &[Vec2],
    source_pad: &LandingPadSpec,
    target_pad: &LandingPadSpec,
) -> Result<()> {
    TerrainDefinition::Heightfield {
        points_m: points.to_vec(),
    }
    .validate()
    .map_err(anyhow::Error::msg)?;
    let first_x = points
        .first()
        .map(|point| point.x)
        .ok_or_else(|| anyhow!("transfer route terrain has no points"))?;
    let last_x = points
        .last()
        .map(|point| point.x)
        .ok_or_else(|| anyhow!("transfer route terrain has no points"))?;
    if first_x > source_pad.center_x_m || last_x < target_pad.center_x_m {
        bail!("transfer route terrain does not contain source-to-target route domain");
    }
    let route_sign = (target_pad.surface_y_m - source_pad.surface_y_m).signum();
    if route_sign.abs() > f64::EPSILON {
        for pair in points.windows(2) {
            let delta_y = pair[1].y - pair[0].y;
            if delta_y.signum() != route_sign && delta_y.abs() > 1e-9 {
                bail!("transfer route terrain must be monotonic between source and target");
            }
        }
    }
    Ok(())
}

pub(super) fn resolve_terminal_matrix_runs(
    entry: &TerminalMatrixEntry,
    base_dir: &Path,
    max_time_s: Option<f64>,
) -> Result<Vec<ResolvedBatchRun>> {
    let base_path = base_dir.join(&entry.base_scenario);
    let base_scenario = load_scenario(&base_path)?;
    let family_spec = terminal_arrival_family_spec(&entry.terminal_matrix)?;
    let arc_specs = selected_terminal_arc_specs(entry, family_spec)?;
    let seed_specs = terminal_seed_specs(entry.seed_tier);
    let mut runs = Vec::new();

    for lane in &entry.lanes {
        let controller_spec = load_controller_spec(
            base_dir,
            lane.controller.as_str(),
            lane.controller_config.as_deref(),
        )?;
        for arc in &arc_specs {
            for band in TERMINAL_BANDS {
                for seed_spec in seed_specs {
                    let run_id = resolved_terminal_matrix_run_id(
                        &entry.id,
                        arc.id,
                        band.id,
                        seed_spec.index,
                        &lane.id,
                    );
                    let (scenario, resolved_parameters, selector) =
                        resolve_terminal_matrix_scenario(TerminalMatrixScenarioRequest {
                            entry,
                            base_scenario: &base_scenario,
                            family_spec,
                            arc,
                            band,
                            seed_spec,
                            lane_id: &lane.id,
                            run_id: &run_id,
                            max_time_s,
                        })?;
                    let descriptor = ResolvedRunDescriptor {
                        run_id,
                        entry_id: entry.id.clone(),
                        source_kind: ResolvedRunSourceKind::TerminalMatrix,
                        scenario_source: entry.base_scenario.clone(),
                        resolved_scenario_id: scenario.id.clone(),
                        resolved_scenario_name: scenario.name.clone(),
                        family_id: Some(entry.id.clone()),
                        selector,
                        lane_id: lane.id.clone(),
                        resolved_seed: seed_spec.index,
                        resolved_parameters,
                        controller_id: controller_spec.id().to_owned(),
                        controller_spec: controller_spec.clone(),
                    };
                    runs.push(ResolvedBatchRun {
                        descriptor,
                        scenario,
                    });
                }
            }
        }
    }

    Ok(runs)
}

pub(super) fn selected_terminal_arc_specs<'a>(
    entry: &TerminalMatrixEntry,
    family_spec: &'a TerminalArrivalFamilySpec,
) -> Result<Vec<&'a TerminalArcPointSpec>> {
    if entry.arc_points.is_empty() {
        return Ok(family_spec.arc_points.iter().collect());
    }

    entry
        .arc_points
        .iter()
        .map(|arc_point| {
            family_spec
                .arc_points
                .iter()
                .find(|candidate| candidate.id == arc_point)
                .with_context(|| {
                    format!(
                        "terminal matrix entry '{}' arc_point selector '{}' is not supported by matrix '{}'",
                        entry.id, arc_point, entry.terminal_matrix
                    )
                })
        })
        .collect()
}

pub(super) fn terminal_arrival_family_spec(
    name: &str,
) -> Result<&'static TerminalArrivalFamilySpec> {
    match name {
        "half_arc_terminal_v1" => Ok(&HALF_ARC_TERMINAL_V1_SPEC),
        _ => bail!("unsupported terminal matrix '{}'", name),
    }
}

pub(super) fn terminal_seed_specs(seed_tier: TerminalSeedTier) -> &'static [TerminalSeedSpec] {
    match seed_tier {
        TerminalSeedTier::Smoke => &TERMINAL_SMOKE_SEEDS,
        TerminalSeedTier::Full => &TERMINAL_FULL_SEEDS,
    }
}

pub(super) fn terminal_condition_spec(condition_set: &str) -> Result<TerminalConditionSpec> {
    match condition_set {
        "clean" => Ok(TerminalConditionSpec::Clean),
        "traj_undershoot_small" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Undershoot,
                severity: "small",
                magnitudes_m: [30.0, 45.0, 60.0],
            },
        )),
        "traj_undershoot_large" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Undershoot,
                severity: "large",
                magnitudes_m: [75.0, 90.0, 105.0],
            },
        )),
        "traj_overshoot_small" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Overshoot,
                severity: "small",
                magnitudes_m: [30.0, 45.0, 60.0],
            },
        )),
        "traj_overshoot_large" => Ok(TerminalConditionSpec::ProjectedError(
            TerminalProjectedErrorSpec {
                kind: TerminalProjectedErrorKind::Overshoot,
                severity: "large",
                magnitudes_m: [75.0, 90.0, 105.0],
            },
        )),
        "terrain_backstop_wall" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::ContainmentBackstop,
                variant: "wall",
                height_offset_m: 400.0,
                pad_clearance_gap_m: 30.0,
                shoulder_width_m: 8.0,
                top_width_m: 120.0,
            },
        )),
        "terrain_backstop_slanted" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::ContainmentBackstop,
                variant: "slanted",
                height_offset_m: 400.0,
                pad_clearance_gap_m: 30.0,
                shoulder_width_m: 90.0,
                top_width_m: 70.0,
            },
        )),
        "terrain_clip" => Ok(TerminalConditionSpec::ReactiveTerrain(
            TerminalReactiveTerrainSpec {
                hazard: TerminalReactiveTerrainHazard::DescentClip,
                variant: "clip",
                height_offset_m: 220.0,
                pad_clearance_gap_m: 24.0,
                shoulder_width_m: 36.0,
                top_width_m: 34.0,
            },
        )),
        _ => bail!("unsupported condition_set '{condition_set}'"),
    }
}

pub(super) fn resolved_terminal_matrix_run_id(
    entry_id: &str,
    arc_point: &str,
    velocity_band: &str,
    seed: u64,
    lane_id: &str,
) -> String {
    sanitize_token(&format!(
        "{entry_id}__{arc_point}__{velocity_band}__seed_{seed:02}__{lane_id}"
    ))
}

#[derive(Clone, Copy)]
pub(super) struct TerminalMatrixScenarioRequest<'a> {
    entry: &'a TerminalMatrixEntry,
    base_scenario: &'a ScenarioSpec,
    family_spec: &'a TerminalArrivalFamilySpec,
    arc: &'a TerminalArcPointSpec,
    band: TerminalBandSpec,
    seed_spec: &'a TerminalSeedSpec,
    lane_id: &'a str,
    run_id: &'a str,
    max_time_s: Option<f64>,
}

pub(super) fn resolve_terminal_matrix_scenario(
    request: TerminalMatrixScenarioRequest<'_>,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>, SelectorAxes)> {
    let TerminalMatrixScenarioRequest {
        entry,
        base_scenario,
        family_spec,
        arc,
        band,
        seed_spec,
        lane_id,
        run_id,
        max_time_s,
    } = request;
    let mut scenario = base_scenario.clone();
    scenario.id = run_id.to_owned();
    scenario.name = format!(
        "{} [{} {} {} {} {} seed {} {}]",
        base_scenario.name,
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.description = format!(
        "{} ({} {} {} {} {} {} seed {} lane {})",
        base_scenario.description,
        "terminal_matrix",
        family_spec.arrival_family,
        entry.condition_set,
        entry.vehicle_variant,
        arc.id,
        band.id,
        seed_spec.index,
        lane_id
    );
    scenario.seed = seed_spec.index;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed_spec.index.to_string());
    scenario
        .metadata
        .insert("mission".to_owned(), "terminal_guidance".to_owned());
    scenario.metadata.insert(
        "arrival_family".to_owned(),
        family_spec.arrival_family.to_owned(),
    );
    scenario
        .metadata
        .insert("condition_set".to_owned(), entry.condition_set.clone());
    scenario
        .metadata
        .insert("vehicle_variant".to_owned(), entry.vehicle_variant.clone());
    scenario.metadata.insert(
        "expectation_tier".to_owned(),
        entry.expectation_tier.clone(),
    );
    scenario
        .metadata
        .insert("arc_point".to_owned(), arc.id.to_owned());
    scenario
        .metadata
        .insert("velocity_band".to_owned(), band.id.to_owned());
    scenario
        .metadata
        .insert("lane_id".to_owned(), lane_id.to_owned());

    scenario.world.gravity_mps2 = family_spec.gravity_mps2;
    let reachability_max_time_s = scenario.sim.max_time_s;
    scenario.metadata.insert(
        "resolved.reachability_max_time_s".to_owned(),
        format!("{:.6}", reachability_max_time_s),
    );
    if let Some(eval_max_time_s) = max_time_s {
        if eval_max_time_s < reachability_max_time_s {
            bail!(
                "terminal_matrix_max_time_s ({eval_max_time_s:.3}) must be >= scenario reachability max_time_s ({reachability_max_time_s:.3}) for terminal matrix entry '{}'",
                entry.id
            );
        }
        scenario.sim.max_time_s = eval_max_time_s;
        scenario.metadata.insert(
            "resolved.eval_max_time_s".to_owned(),
            format!("{:.6}", eval_max_time_s),
        );
    }

    let mut resolved_parameters = BTreeMap::new();
    resolved_parameters.insert("gravity_mps2".to_owned(), family_spec.gravity_mps2);
    resolved_parameters.insert(
        "reachability_max_time_s".to_owned(),
        reachability_max_time_s,
    );
    if let Some(eval_max_time_s) = max_time_s {
        resolved_parameters.insert("eval_max_time_s".to_owned(), eval_max_time_s);
    }
    resolved_parameters.insert("radius_nominal_m".to_owned(), family_spec.radius_nominal_m);
    resolved_parameters.insert("arc_angle_deg".to_owned(), arc.angle_deg);
    resolved_parameters.insert("mid_ttg_s".to_owned(), arc.nominal_ttg_s);

    for adjustment in &entry.adjustments {
        apply_numeric_adjustment(&mut scenario, adjustment)?;
        resolved_parameters.insert(adjustment.id.clone(), adjustment.value);
        scenario.metadata.insert(
            format!("resolved.{}", adjustment.id),
            format!("{:.6}", adjustment.value),
        );
    }

    let ttg_s = terminal_band_ttg(family_spec, arc, band.id);
    resolved_parameters.insert("ttg_s".to_owned(), ttg_s);

    let (side_label, side_sign) = resolved_side(arc.id, seed_spec.index);
    scenario
        .metadata
        .insert("resolved.side".to_owned(), side_label.to_owned());
    resolved_parameters.insert("side_sign".to_owned(), side_sign);

    let radial_jitter_m = seed_spec
        .radial_pct
        .map(|pct| (family_spec.radius_nominal_m * pct).clamp(-30.0, 30.0))
        .unwrap_or(0.0);
    let resolved_radius_m = family_spec.radius_nominal_m + radial_jitter_m;
    resolved_parameters.insert("radial_jitter_m".to_owned(), radial_jitter_m);
    resolved_parameters.insert("radius_m".to_owned(), resolved_radius_m);
    if let Some(radial_pct) = seed_spec.radial_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "radial".to_owned());
        scenario
            .metadata
            .insert("resolved.radial_pct".to_owned(), format!("{radial_pct:.6}"));
    } else if let Some(speed_pct) = seed_spec.speed_pct {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "speed".to_owned());
        scenario
            .metadata
            .insert("resolved.speed_pct".to_owned(), format!("{speed_pct:.6}"));
    } else {
        scenario
            .metadata
            .insert("resolved.seed_variation".to_owned(), "none".to_owned());
    }

    let angle_rad = arc.angle_deg.to_radians();
    let x_m = if arc.id == "a00" {
        0.0
    } else {
        side_sign * resolved_radius_m * angle_rad.sin()
    };
    let y_m = resolved_radius_m * angle_rad.cos();
    resolved_parameters.insert("start_x_m".to_owned(), x_m);
    resolved_parameters.insert("start_y_m".to_owned(), y_m);

    let (clean_vx_mps, clean_vy_mps) =
        solve_ballistic_velocity(x_m, y_m, ttg_s, family_spec.gravity_mps2);
    let condition_spec = terminal_condition_spec(&entry.condition_set)?;
    scenario.metadata.insert(
        "resolved.condition_kind".to_owned(),
        condition_spec.kind_label().to_owned(),
    );
    let mut vx_mps = clean_vx_mps;
    let mut vy_mps = clean_vy_mps;
    let mut speed_scale = 1.0;
    let mut projected_dx_error_m = 0.0;
    let mut projected_dx_error_mag_m = 0.0;
    let mut traj_error_approach_sign = if x_m.abs() > f64::EPSILON {
        x_m.signum()
    } else if seed_spec.index.is_multiple_of(2) {
        -1.0
    } else {
        1.0
    };

    if let TerminalConditionSpec::ProjectedError(error_spec) = condition_spec {
        let magnitude_index = seed_spec
            .error_level_index
            .min(error_spec.magnitudes_m.len().saturating_sub(1));
        projected_dx_error_mag_m = error_spec.magnitudes_m[magnitude_index];
        let error_sign = match error_spec.kind {
            TerminalProjectedErrorKind::Undershoot => traj_error_approach_sign,
            TerminalProjectedErrorKind::Overshoot => -traj_error_approach_sign,
        };
        projected_dx_error_m = error_sign * projected_dx_error_mag_m;
        vx_mps = (projected_dx_error_m - x_m) / ttg_s;
        scenario.metadata.insert(
            "resolved.traj_error_kind".to_owned(),
            error_spec.kind.as_str().to_owned(),
        );
        scenario.metadata.insert(
            "resolved.traj_error_severity".to_owned(),
            error_spec.severity.to_owned(),
        );
        scenario.metadata.insert(
            "resolved.seed_variation".to_owned(),
            "projected_error".to_owned(),
        );
        scenario.metadata.remove("resolved.speed_pct");
    } else {
        traj_error_approach_sign = 0.0;
        speed_scale = 1.0 + seed_spec.speed_pct.unwrap_or(0.0);
        vx_mps *= speed_scale;
        vy_mps *= speed_scale;
        scenario
            .metadata
            .insert("resolved.traj_error_kind".to_owned(), "none".to_owned());
        scenario
            .metadata
            .insert("resolved.traj_error_severity".to_owned(), "none".to_owned());
    }
    let engine_off_impact_x_m = x_m + (vx_mps * ttg_s);
    resolved_parameters.insert("clean_start_vx_mps".to_owned(), clean_vx_mps);
    resolved_parameters.insert("clean_start_vy_mps".to_owned(), clean_vy_mps);
    resolved_parameters.insert("projected_dx_error_m".to_owned(), projected_dx_error_m);
    resolved_parameters.insert(
        "projected_dx_error_mag_m".to_owned(),
        projected_dx_error_mag_m,
    );
    resolved_parameters.insert(
        "traj_error_approach_sign".to_owned(),
        traj_error_approach_sign,
    );
    resolved_parameters.insert("engine_off_impact_x_m".to_owned(), engine_off_impact_x_m);
    resolved_parameters.insert("speed_scale".to_owned(), speed_scale);
    resolved_parameters.insert("start_vx_mps".to_owned(), vx_mps);
    resolved_parameters.insert("start_vy_mps".to_owned(), vy_mps);
    resolved_parameters.insert(
        "start_speed_mps".to_owned(),
        (vx_mps.powi(2) + vy_mps.powi(2)).sqrt(),
    );

    scenario.initial_state.position_m.x = x_m;
    scenario.initial_state.position_m.y = y_m;
    scenario.initial_state.velocity_mps.x = vx_mps;
    scenario.initial_state.velocity_mps.y = vy_mps;

    if let TerminalConditionSpec::ReactiveTerrain(terrain_spec) = condition_spec {
        apply_terminal_reactive_terrain(
            &mut scenario,
            terrain_spec,
            side_sign,
            seed_spec.index,
            &mut resolved_parameters,
        )?;
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved terminal matrix scenario '{}' {} {} seed {} failed validation",
                entry.id, arc.id, band.id, seed_spec.index
            )
        })?;

    let selector = SelectorAxes {
        mission: "terminal_guidance".to_owned(),
        arrival_family: family_spec.arrival_family.to_owned(),
        condition_set: entry.condition_set.clone(),
        vehicle_variant: entry.vehicle_variant.clone(),
        arc_point: arc.id.to_owned(),
        velocity_band: band.id.to_owned(),
        route_family: default_selector_value(),
        route_angle: default_selector_value(),
        radius_tier: default_selector_value(),
        waypoint_profile: default_selector_value(),
        waypoint_handoff_envelope: default_selector_value(),
        expectation_tier: Some(entry.expectation_tier.clone()),
    };

    Ok((scenario, resolved_parameters, selector))
}

pub(super) fn terminal_band_ttg(
    family_spec: &TerminalArrivalFamilySpec,
    arc: &TerminalArcPointSpec,
    band_id: &str,
) -> f64 {
    match band_id {
        "mid" => arc.nominal_ttg_s,
        "low" => {
            let low = arc.nominal_ttg_s * family_spec.low_multiplier;
            if family_spec.clamp_low_to_descending {
                let y_m = family_spec.radius_nominal_m * arc.angle_deg.to_radians().cos();
                let t_flat_s = ((2.0 * y_m) / family_spec.gravity_mps2).sqrt();
                low.min(t_flat_s * 0.98)
            } else {
                low
            }
        }
        "high" => arc.nominal_ttg_s * family_spec.high_multiplier,
        _ => arc.nominal_ttg_s,
    }
}

pub(super) fn resolved_side(arc_point: &str, seed: u64) -> (&'static str, f64) {
    if arc_point == "a00" {
        ("center", 0.0)
    } else if seed.is_multiple_of(2) {
        ("left", -1.0)
    } else {
        ("right", 1.0)
    }
}

pub(super) fn solve_ballistic_velocity(
    x_m: f64,
    y_m: f64,
    ttg_s: f64,
    gravity_mps2: f64,
) -> (f64, f64) {
    let vx_mps = -x_m / ttg_s;
    let vy_mps = ((0.5 * gravity_mps2 * ttg_s * ttg_s) - y_m) / ttg_s;
    (vx_mps, vy_mps)
}

pub(super) fn apply_terminal_reactive_terrain(
    scenario: &mut ScenarioSpec,
    terrain_spec: TerminalReactiveTerrainSpec,
    side_sign: f64,
    seed_index: u64,
    resolved_parameters: &mut BTreeMap<String, f64>,
) -> Result<()> {
    let Some(target_pad) = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())
    else {
        bail!(
            "terminal reactive terrain condition requires target pad '{}'",
            scenario.mission.goal.target_pad_id()
        );
    };
    let target_center_x_m = target_pad.center_x_m;
    let target_surface_y_m = target_pad.surface_y_m;
    let pad_half_width_m = target_pad.half_width_m();
    let approach_side_sign = terminal_terrain_approach_side_sign(side_sign, seed_index);
    let feature_side_sign = match terrain_spec.hazard {
        TerminalReactiveTerrainHazard::ContainmentBackstop => -approach_side_sign,
        TerminalReactiveTerrainHazard::DescentClip => approach_side_sign,
    };
    let inner_offset_m = pad_half_width_m + terrain_spec.pad_clearance_gap_m;
    let far_offset_m = (scenario.initial_state.position_m.x - target_center_x_m)
        .abs()
        .max(900.0)
        + 240.0;
    let terrain_points = match terrain_spec.hazard {
        TerminalReactiveTerrainHazard::ContainmentBackstop => terminal_backstop_profile_points(
            target_center_x_m,
            target_surface_y_m,
            feature_side_sign,
            far_offset_m,
            inner_offset_m,
            terrain_spec,
        ),
        TerminalReactiveTerrainHazard::DescentClip => terminal_clip_profile_points(
            target_center_x_m,
            target_surface_y_m,
            feature_side_sign,
            far_offset_m,
            inner_offset_m,
            terrain_spec,
        ),
    }?;

    scenario.world.terrain = TerrainDefinition::Heightfield {
        points_m: terrain_points,
    };
    scenario.metadata.insert(
        "resolved.reactive_contract".to_owned(),
        "execution_guardrail".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.reactive_trigger".to_owned(),
        "execution_drift".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.primary_navigation_owner".to_owned(),
        "terminal_guidance".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.nominal_route_must_clear".to_owned(),
        "true".to_owned(),
    );
    scenario.metadata.insert(
        "resolved.hazard_driver".to_owned(),
        terrain_spec.hazard.as_str().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.obstacle_kind".to_owned(),
        terrain_spec.hazard.obstacle_kind().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.obstacle_placement".to_owned(),
        terrain_spec.hazard.obstacle_placement().to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_variant".to_owned(),
        terrain_spec.variant.to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_feature_side".to_owned(),
        side_label_for_sign(feature_side_sign).to_owned(),
    );
    scenario.metadata.insert(
        "resolved.terrain_visibility".to_owned(),
        "startup_context".to_owned(),
    );

    resolved_parameters.insert("terrain_feature_side_sign".to_owned(), feature_side_sign);
    resolved_parameters.insert("terrain_approach_side_sign".to_owned(), approach_side_sign);
    resolved_parameters.insert(
        "terrain_height_offset_m".to_owned(),
        terrain_spec.height_offset_m,
    );
    resolved_parameters.insert("terrain_inner_offset_m".to_owned(), inner_offset_m);
    resolved_parameters.insert(
        "terrain_pad_clearance_gap_m".to_owned(),
        terrain_spec.pad_clearance_gap_m,
    );
    resolved_parameters.insert(
        "terrain_shoulder_width_m".to_owned(),
        terrain_spec.shoulder_width_m,
    );
    resolved_parameters.insert("terrain_top_width_m".to_owned(), terrain_spec.top_width_m);
    resolved_parameters.insert("terrain_far_offset_m".to_owned(), far_offset_m);

    Ok(())
}

pub(super) fn terminal_terrain_approach_side_sign(side_sign: f64, seed_index: u64) -> f64 {
    if side_sign.abs() > f64::EPSILON {
        side_sign.signum()
    } else if seed_index.is_multiple_of(2) {
        -1.0
    } else {
        1.0
    }
}

pub(super) fn side_label_for_sign(side_sign: f64) -> &'static str {
    if side_sign < 0.0 { "left" } else { "right" }
}

pub(super) fn terminal_backstop_profile_points(
    target_center_x_m: f64,
    target_surface_y_m: f64,
    feature_side_sign: f64,
    far_offset_m: f64,
    inner_offset_m: f64,
    terrain_spec: TerminalReactiveTerrainSpec,
) -> Result<Vec<Vec2>> {
    let ramp_end_m = inner_offset_m + terrain_spec.shoulder_width_m;
    let plateau_end_m = ramp_end_m + terrain_spec.top_width_m;
    terrain_points_from_signed_profile(
        target_center_x_m,
        feature_side_sign,
        &[
            (-far_offset_m, target_surface_y_m),
            (inner_offset_m, target_surface_y_m),
            (
                ramp_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                plateau_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                far_offset_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
        ],
    )
}

pub(super) fn terminal_clip_profile_points(
    target_center_x_m: f64,
    target_surface_y_m: f64,
    feature_side_sign: f64,
    far_offset_m: f64,
    inner_offset_m: f64,
    terrain_spec: TerminalReactiveTerrainSpec,
) -> Result<Vec<Vec2>> {
    let ramp_up_end_m = inner_offset_m + terrain_spec.shoulder_width_m;
    let plateau_end_m = ramp_up_end_m + terrain_spec.top_width_m;
    let outer_end_m = plateau_end_m + terrain_spec.shoulder_width_m;
    terrain_points_from_signed_profile(
        target_center_x_m,
        feature_side_sign,
        &[
            (-far_offset_m, target_surface_y_m),
            (inner_offset_m, target_surface_y_m),
            (
                ramp_up_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (
                plateau_end_m,
                target_surface_y_m + terrain_spec.height_offset_m,
            ),
            (outer_end_m, target_surface_y_m),
            (far_offset_m, target_surface_y_m),
        ],
    )
}

pub(super) fn terrain_points_from_signed_profile(
    target_center_x_m: f64,
    feature_side_sign: f64,
    signed_profile: &[(f64, f64)],
) -> Result<Vec<Vec2>> {
    let mut points: Vec<Vec2> = signed_profile
        .iter()
        .map(|(signed_offset_m, y_m)| {
            Vec2::new(
                target_center_x_m + (feature_side_sign * signed_offset_m),
                *y_m,
            )
        })
        .collect();
    points.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for pair in points.windows(2) {
        if pair[1].x <= pair[0].x {
            bail!("terminal reactive terrain produced duplicate terrain x coordinates");
        }
    }
    Ok(points)
}

pub(super) fn family_entry_seeds(entry: &ScenarioFamilyEntry) -> Vec<u64> {
    if !entry.seeds.is_empty() {
        return entry.seeds.clone();
    }

    let range = entry
        .seed_range
        .as_ref()
        .expect("validated family entries always define seed source");
    (range.start..range.start.saturating_add(range.count)).collect()
}

pub(super) fn resolve_family_scenario(
    entry: &ScenarioFamilyEntry,
    base_scenario: &ScenarioSpec,
    seed: u64,
) -> Result<(ScenarioSpec, BTreeMap<String, f64>)> {
    let mut scenario = base_scenario.clone();
    scenario.id = resolved_family_run_id(&entry.id, seed);
    scenario.name = format!("{} [{} seed {}]", base_scenario.name, entry.family, seed);
    scenario.description = format!(
        "{} (family {} seed {})",
        base_scenario.description, entry.family, seed
    );
    scenario.seed = seed;
    scenario.tags = merge_unique_tags(&base_scenario.tags, &entry.tags);
    scenario.metadata.extend(entry.metadata.clone());
    scenario
        .metadata
        .insert("family".to_owned(), entry.family.clone());
    scenario
        .metadata
        .insert("family_entry_id".to_owned(), entry.id.clone());
    scenario
        .metadata
        .insert("resolved_seed".to_owned(), seed.to_string());

    let mut resolved_parameters = BTreeMap::new();
    for perturbation in &entry.perturbations {
        let sampled_value = sample_perturbation_value(entry, perturbation, seed);
        apply_numeric_perturbation(&mut scenario, perturbation, sampled_value)?;
        resolved_parameters.insert(perturbation.id.clone(), sampled_value);
        scenario.metadata.insert(
            format!("resolved.{}", perturbation.id),
            format!("{sampled_value:.6}"),
        );
    }

    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "resolved family scenario '{}' seed {} failed validation",
                entry.family, seed
            )
        })?;

    Ok((scenario, resolved_parameters))
}

pub(super) fn merge_unique_tags(base_tags: &[String], extra_tags: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(base_tags.len() + extra_tags.len());
    let mut seen = BTreeSet::new();
    for tag in base_tags.iter().chain(extra_tags.iter()) {
        if seen.insert(tag.clone()) {
            merged.push(tag.clone());
        }
    }
    merged
}

pub(super) fn selector_axes_from_metadata(metadata: &BTreeMap<String, String>) -> SelectorAxes {
    SelectorAxes {
        mission: selector_value(metadata.get("mission"), "unspecified"),
        arrival_family: selector_value(metadata.get("arrival_family"), "unspecified"),
        condition_set: selector_value(metadata.get("condition_set"), "unspecified"),
        vehicle_variant: selector_value(metadata.get("vehicle_variant"), "unspecified"),
        arc_point: selector_value(metadata.get("arc_point"), "unspecified"),
        velocity_band: selector_value(metadata.get("velocity_band"), "unspecified"),
        route_family: selector_value(metadata.get("route_family"), "unspecified"),
        route_angle: selector_value(metadata.get("route_angle"), "unspecified"),
        radius_tier: selector_value(metadata.get("radius_tier"), "unspecified"),
        waypoint_profile: selector_value(
            metadata
                .get("waypoint_profile")
                .or_else(|| metadata.get("route_mode")),
            "unspecified",
        ),
        waypoint_handoff_envelope: selector_value(
            metadata.get("waypoint_handoff_envelope"),
            "unspecified",
        ),
        expectation_tier: metadata
            .get("expectation_tier")
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
    }
}

pub(super) fn selector_value(value: Option<&String>, fallback: &str) -> String {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

pub(super) fn is_supported_numeric_path(path: &str) -> bool {
    matches!(
        path,
        "world.gravity_mps2"
            | "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.position_m.x"
            | "initial_state.position_m.y"
            | "initial_state.velocity_mps.x"
            | "initial_state.velocity_mps.y"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

pub(super) fn is_supported_terminal_adjustment_path(path: &str) -> bool {
    matches!(
        path,
        "vehicle.dry_mass_kg"
            | "vehicle.initial_fuel_kg"
            | "vehicle.max_fuel_kg"
            | "vehicle.max_thrust_n"
            | "vehicle.max_fuel_burn_kgps"
            | "vehicle.max_rotation_rate_radps"
            | "initial_state.attitude_rad"
            | "initial_state.angular_rate_radps"
    )
}

pub(super) fn apply_numeric_perturbation(
    scenario: &mut ScenarioSpec,
    perturbation: &NumericPerturbationSpec,
    sampled_value: f64,
) -> Result<()> {
    let Some(target) = scenario_numeric_target_mut(scenario, &perturbation.path) else {
        bail!(
            "unsupported numeric perturbation path '{}'",
            perturbation.path
        );
    };
    apply_numeric_mode(target, perturbation.mode, sampled_value);
    Ok(())
}

pub(super) fn apply_numeric_adjustment(
    scenario: &mut ScenarioSpec,
    adjustment: &NumericAdjustmentSpec,
) -> Result<()> {
    if !is_supported_terminal_adjustment_path(&adjustment.path) {
        bail!("unsupported numeric adjustment path '{}'", adjustment.path);
    }
    let target = scenario_numeric_target_mut(scenario, &adjustment.path)
        .expect("validated numeric adjustment paths must resolve to scenario fields");
    apply_numeric_mode(target, adjustment.mode, adjustment.value);
    Ok(())
}

pub(super) fn scenario_numeric_target_mut<'a>(
    scenario: &'a mut ScenarioSpec,
    path: &str,
) -> Option<&'a mut f64> {
    match path {
        "world.gravity_mps2" => Some(&mut scenario.world.gravity_mps2),
        "vehicle.dry_mass_kg" => Some(&mut scenario.vehicle.dry_mass_kg),
        "vehicle.initial_fuel_kg" => Some(&mut scenario.vehicle.initial_fuel_kg),
        "vehicle.max_fuel_kg" => Some(&mut scenario.vehicle.max_fuel_kg),
        "vehicle.max_thrust_n" => Some(&mut scenario.vehicle.max_thrust_n),
        "vehicle.max_fuel_burn_kgps" => Some(&mut scenario.vehicle.max_fuel_burn_kgps),
        "vehicle.max_rotation_rate_radps" => Some(&mut scenario.vehicle.max_rotation_rate_radps),
        "initial_state.position_m.x" => Some(&mut scenario.initial_state.position_m.x),
        "initial_state.position_m.y" => Some(&mut scenario.initial_state.position_m.y),
        "initial_state.velocity_mps.x" => Some(&mut scenario.initial_state.velocity_mps.x),
        "initial_state.velocity_mps.y" => Some(&mut scenario.initial_state.velocity_mps.y),
        "initial_state.attitude_rad" => Some(&mut scenario.initial_state.attitude_rad),
        "initial_state.angular_rate_radps" => Some(&mut scenario.initial_state.angular_rate_radps),
        _ => None,
    }
}

pub(super) fn apply_numeric_mode(target: &mut f64, mode: NumericPerturbationMode, value: f64) {
    match mode {
        NumericPerturbationMode::Set => *target = value,
        NumericPerturbationMode::Offset => *target += value,
        NumericPerturbationMode::Scale => *target *= value,
    }
}

pub(super) fn sample_perturbation_value(
    entry: &ScenarioFamilyEntry,
    perturbation: &NumericPerturbationSpec,
    seed: u64,
) -> f64 {
    let random_value = stable_unit_interval(
        seed,
        &format!("{}::{}::{}", entry.family, entry.id, perturbation.id),
    );
    let sampled = perturbation.min + ((perturbation.max - perturbation.min) * random_value);
    if let Some(step) = perturbation.quantize {
        (sampled / step).round() * step
    } else {
        sampled
    }
}
