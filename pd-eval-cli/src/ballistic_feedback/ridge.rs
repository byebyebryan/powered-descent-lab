//! First-feature placement only. This is neither flight admission nor a search
//! for a higher destination arc. Small terrain dips do not end a blocking ridge.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementKind {
    BlockingRidge,
    LocalClimbStage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RidgeSelection {
    pub kind: PlacementKind,
    pub conflict_x_m: f64,
    pub scan_start_x_m: f64,
    pub scan_end_x_m: f64,
    pub crest_m: Option<Vec2>,
    pub separating_drop_m: f64,
    pub diameter_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_height: Option<LocalHeight>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalHeight {
    pub terrain_height_m: f64,
    pub incoming_corridor_max_m: f64,
    pub allowance_m: f64,
}

/// Resolve the first forward crest with one body-scale drop hysteresis. A deep
/// valley ends this feature; tiny dips and equal-height plateau points do not.
/// Unresolved slopes keep the old bounded progress goal, explicitly as a stage.
pub(super) fn placement(
    terrain: &pd_core::TerrainDefinition,
    current_x: f64,
    conflict_x: f64,
    target_x: f64,
    diameter: f64,
) -> Result<(f64, RidgeSelection)> {
    ensure!(
        [current_x, conflict_x, target_x, diameter]
            .iter()
            .all(|v| v.is_finite())
            && diameter > 0.0,
        "nonfinite ridge placement query"
    );
    let fallback_x = conflict_x.max(current_x) + diameter;
    ensure!(
        fallback_x < target_x,
        "obstruction goal would pass destination"
    );
    let start_x = current_x.max(conflict_x - diameter * 0.5);
    let limit_x = target_x - diameter;
    let mut crest = Vec2::new(start_x, terrain.sample_height_strict(start_x)?);
    let mut end_x = start_x;
    let mut drop = 0.0;
    let mut resolved = false;
    if start_x < limit_x {
        // Include an interpolated end boundary; never clamp an invalid domain.
        let boundary = Vec2::new(limit_x, terrain.sample_height_strict(limit_x)?);
        for p in terrain
            .points()
            .iter()
            .copied()
            .filter(|p| p.x > start_x && p.x < limit_x)
            .chain(std::iter::once(boundary))
        {
            end_x = p.x;
            if p.y >= crest.y {
                crest = p;
            }
            drop = crest.y - p.y;
            if drop >= diameter {
                resolved = true;
                break;
            }
        }
    }
    let x = if resolved {
        (crest.x + diameter).max(current_x + diameter)
    } else {
        fallback_x
    };
    // The maximum H-window early crossing still puts the conservative body
    // behind the crest. No actual handoff predicate or reserve is relaxed.
    ensure!(x < target_x, "ridge goal would pass destination");
    Ok((
        x,
        RidgeSelection {
            kind: if resolved {
                PlacementKind::BlockingRidge
            } else {
                PlacementKind::LocalClimbStage
            },
            conflict_x_m: conflict_x,
            scan_start_x_m: start_x,
            scan_end_x_m: end_x,
            crest_m: resolved.then_some(crest),
            separating_drop_m: drop,
            diameter_m: diameter,
            local_height: None,
        },
    ))
}

#[cfg(test)]
pub(super) fn waypoint(
    ctx: &RunContext,
    state: &SimulationState,
    conflict: Vec2,
    number: usize,
) -> Result<(Goal, RidgeSelection)> {
    waypoint_for(ctx, state, conflict, number, false)
}

pub(super) fn waypoint_for(
    ctx: &RunContext,
    state: &SimulationState,
    conflict: Vec2,
    number: usize,
    local_height: bool,
) -> Result<(Goal, RidgeSelection)> {
    let diameter = conservative_body_diameter(&ctx.vehicle.geometry);
    let (x, mut selection) = placement(
        &ctx.world.terrain,
        state.position_m.x,
        conflict.x,
        ctx.target_pad.center_x_m,
        diameter,
    )?;
    let mut height = ctx.world.terrain.sample_height_strict(x)?;
    for p in ctx.world.terrain.points() {
        if p.x >= state.position_m.x && p.x <= x {
            height = height.max(p.y);
        }
    }
    let allowance = diameter * 0.5
        + 5.0
        + 0.5
            * ctx.world.gravity_mps2
            * (CONTINUATION_TICKS as f64 * ctx.sim.physics_dt_s()).powi(2);
    if local_height {
        let corridor_max = height;
        height = ctx.world.terrain.sample_height_strict(x)?;
        if let Some(crest) = selection.crest_m {
            height = height.max(crest.y);
        } else {
            for p in ctx.world.terrain.points() {
                if p.x >= selection.scan_start_x_m && p.x <= x {
                    height = height.max(p.y);
                }
            }
            height = height.max(
                ctx.world
                    .terrain
                    .sample_height_strict(selection.scan_start_x_m)?,
            );
        }
        selection.local_height = Some(LocalHeight {
            terrain_height_m: height,
            incoming_corridor_max_m: corridor_max,
            allowance_m: allowance,
        });
    }
    let y = if local_height {
        height + allowance
    } else {
        // Preserve the exact floating-point grouping of all earlier modes.
        height
            + diameter * 0.5
            + 5.0
            + 0.5
                * ctx.world.gravity_mps2
                * (CONTINUATION_TICKS as f64 * ctx.sim.physics_dt_s()).powi(2)
    };
    Ok((
        Goal {
            position_m: Vec2::new(x, y),
            destination: false,
            number,
            revision: 0,
        },
        selection,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain(points: &[(f64, f64)]) -> pd_core::TerrainDefinition {
        pd_core::TerrainDefinition::Heightfield {
            points_m: points.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        }
    }

    #[test]
    fn shallow_valley_merges_into_the_807_shaped_ridge() {
        let t = terrain(&[
            (0.0, -177.0),
            (42.0, -103.0),
            (56.0, -91.0),
            (72.0, -97.0),
            (100.0, -25.0),
            (180.0, 508.0),
            (192.0, 489.0),
            (1200.0, -228.0),
        ]);
        for (current, conflict) in [(0.0, 29.0), (41.65, 96.8), (109.6, 132.3)] {
            let (x, selection) = placement(&t, current, conflict, 1200.0, 12.8).unwrap();
            assert_eq!(selection.kind, PlacementKind::BlockingRidge);
            assert_eq!(selection.crest_m, Some(Vec2::new(180.0, 508.0)));
            assert_eq!(x, 192.8);
        }
    }

    #[test]
    fn substantial_valley_does_not_jump_to_the_later_higher_peak() {
        let t = terrain(&[
            (0.0, 0.0),
            (100.0, 100.0),
            (180.0, 0.0),
            (300.0, 500.0),
            (800.0, 0.0),
        ]);
        let (x, selection) = placement(&t, 0.0, 60.0, 800.0, 12.8).unwrap();
        assert_eq!(selection.crest_m, Some(Vec2::new(100.0, 100.0)));
        assert_eq!(x, 112.8);
    }

    #[test]
    fn plateau_uses_the_last_equal_height_crest_point() {
        let t = terrain(&[
            (0.0, 0.0),
            (100.0, 100.0),
            (160.0, 100.0),
            (200.0, 0.0),
            (800.0, 0.0),
        ]);
        let (x, selection) = placement(&t, 0.0, 60.0, 800.0, 12.8).unwrap();
        assert_eq!(selection.crest_m, Some(Vec2::new(160.0, 100.0)));
        assert_eq!(x, 172.8);
    }

    #[test]
    fn monotonic_uphill_is_a_stage_not_a_fictitious_cleared_ridge() {
        let t = terrain(&[(0.0, 0.0), (200.0, 100.0), (800.0, 500.0)]);
        let (x, selection) = placement(&t, 0.0, 60.0, 800.0, 12.8).unwrap();
        assert_eq!(x, 72.8);
        assert_eq!(selection.kind, PlacementKind::LocalClimbStage);
        assert!(selection.crest_m.is_none());
        assert!(selection.scan_end_x_m < 800.0);
    }

    #[test]
    fn domain_and_destination_errors_are_not_hidden_by_fallback() {
        let t = terrain(&[(0.0, 0.0), (100.0, 100.0)]);
        assert!(placement(&t, 0.0, 60.0, 800.0, 12.8).is_err());
        assert!(placement(&t, 0.0, 90.0, 100.0, 12.8).is_err());
        assert!(placement(&t, 0.0, f64::NAN, 100.0, 12.8).is_err());
    }

    #[test]
    fn waypoint_query_keeps_the_plant_unchanged() {
        let mut ctx = RunContext::from_scenario(
            &crate::test_inputs::planner_request("v2_clear_845").scenario,
        )
        .unwrap();
        ctx.target_pad.center_x_m = 800.0;
        ctx.world.terrain = terrain(&[(-1005.0, 0.0), (100.0, 100.0), (200.0, 0.0), (800.0, 0.0)]);
        let mut state = SimulationState::new(&ctx).unwrap();
        state.position_m = Vec2::new(0.0, 20.0);
        let before = SimulationStateSnapshotV1::from_state(&state);
        let (goal, selection) = waypoint(&ctx, &state, Vec2::new(60.0, 50.0), 1).unwrap();
        assert!(!goal.destination);
        assert_eq!(selection.crest_m, Some(Vec2::new(100.0, 100.0)));
        assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    }

    #[test]
    fn local_seed_does_not_exempt_earlier_terrain_from_incoming_arc_checks() {
        let request = crate::test_inputs::planner_request("v2_clear_845");
        let mut ctx = RunContext::from_scenario(&request.scenario).unwrap();
        ctx.target_pad.center_x_m = 1200.0;
        ctx.world.terrain = terrain(&[
            (-1100.0, 0.0),
            (344.0, 130.0),
            (360.0, 500.0),
            (600.0, -250.0),
            (1158.0, 67.0),
            (1168.0, 48.0),
            (1200.0, 0.0),
            (1400.0, 0.0),
        ]);
        let mut state = SimulationState::new(&ctx).unwrap();
        state.position_m = Vec2::new(345.0, 209.0);
        let (goal, selection) =
            waypoint_for(&ctx, &state, Vec2::new(1162.0, 77.0), 2, true).unwrap();
        assert_eq!(selection.local_height.unwrap().terrain_height_m, 67.0);
        let arc =
            aim::target_arc(kinematics(&state), goal.position_m, 9.81, 1.0 / 120.0, 1558).unwrap();
        let warning = first_conflict(&request, &ctx, &state, &arc)
            .unwrap()
            .unwrap();
        assert!(warning.state.position_m.x < 360.0);
    }

    #[test]
    fn local_height_does_not_carry_an_earlier_cleared_shoulder_across_a_valley() {
        let mut ctx = RunContext::from_scenario(
            &crate::test_inputs::planner_request("v2_clear_845").scenario,
        )
        .unwrap();
        ctx.target_pad.center_x_m = 1200.0;
        ctx.world.terrain = terrain(&[
            (-1100.0, 0.0),
            (344.0, 140.0),
            (348.0, 130.0),
            (600.0, -250.0),
            (1158.0, 67.0),
            (1168.0, 48.0),
            (1200.0, 0.0),
            (1400.0, 0.0),
        ]);
        let mut state = SimulationState::new(&ctx).unwrap();
        state.position_m = Vec2::new(345.0, 209.0);
        state.velocity_mps = Vec2::new(25.0, -2.0);
        let before = SimulationStateSnapshotV1::from_state(&state);
        let conflict = Vec2::new(1162.0, 77.0);
        let (old, _) = waypoint(&ctx, &state, conflict, 2).unwrap();
        let (local, selection) = waypoint_for(&ctx, &state, conflict, 2, true).unwrap();
        let basis = selection.local_height.unwrap();
        assert_eq!(local.position_m.x, old.position_m.x);
        assert_eq!(basis.terrain_height_m, 67.0);
        assert_eq!(basis.incoming_corridor_max_m, 130.0);
        assert!((old.position_m.y - local.position_m.y - 63.0).abs() < 1e-10);
        assert_eq!(SimulationStateSnapshotV1::from_state(&state), before);
    }
}
