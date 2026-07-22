use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct LatestSafeState {
    pub(super) latest_safe_margin_s: f64,
    pub(super) best_candidate: TerminalGateCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NominalState {
    pub(super) best_candidate: TerminalGateCandidate,
    pub(super) ready: bool,
}

pub(super) fn candidate_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    let primary = (!lhs.tilt_feasible, !lhs.terrain_clearance_safe)
        .cmp(&(!rhs.tilt_feasible, !rhs.terrain_clearance_safe));
    if primary != std::cmp::Ordering::Equal {
        return primary;
    }
    if !lhs.terrain_clearance_safe && !rhs.terrain_clearance_safe {
        return terrain_constrained_preference_order(lhs, rhs);
    }
    (
        OrderedF64(lhs.required_accel_ratio),
        OrderedF64(lhs.burn_time_s),
        OrderedF64(-lhs.terrain_min_clearance_m),
    )
        .cmp(&(
            OrderedF64(rhs.required_accel_ratio),
            OrderedF64(rhs.burn_time_s),
            OrderedF64(-rhs.terrain_min_clearance_m),
        ))
}

pub(super) fn latest_safe_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    let primary = (!lhs.tilt_feasible, !lhs.terrain_clearance_safe)
        .cmp(&(!rhs.tilt_feasible, !rhs.terrain_clearance_safe));
    if primary != std::cmp::Ordering::Equal {
        return primary;
    }
    if !lhs.terrain_clearance_safe && !rhs.terrain_clearance_safe {
        return terrain_constrained_preference_order(lhs, rhs);
    }
    (
        lhs.required_accel_ratio > 1.0,
        OrderedF64(lhs.burn_time_s),
        OrderedF64(lhs.required_accel_ratio),
        OrderedF64(-lhs.terrain_min_clearance_m),
    )
        .cmp(&(
            rhs.required_accel_ratio > 1.0,
            OrderedF64(rhs.burn_time_s),
            OrderedF64(rhs.required_accel_ratio),
            OrderedF64(-rhs.terrain_min_clearance_m),
        ))
}

pub(super) fn aggressive_latest_safe_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    let primary = (!lhs.tilt_feasible, !lhs.terrain_clearance_safe)
        .cmp(&(!rhs.tilt_feasible, !rhs.terrain_clearance_safe));
    if primary != std::cmp::Ordering::Equal {
        return primary;
    }
    if !lhs.terrain_clearance_safe && !rhs.terrain_clearance_safe {
        return terrain_constrained_preference_order(lhs, rhs);
    }
    (
        OrderedF64(lhs.burn_time_s),
        OrderedF64(lhs.required_accel_ratio),
        OrderedF64(-lhs.terrain_min_clearance_m),
    )
        .cmp(&(
            OrderedF64(rhs.burn_time_s),
            OrderedF64(rhs.required_accel_ratio),
            OrderedF64(-rhs.terrain_min_clearance_m),
        ))
}

pub(super) fn terrain_constrained_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    (
        OrderedF64(-lhs.terrain_min_clearance_m),
        OrderedF64(lhs.required_accel_ratio),
        OrderedF64(lhs.burn_time_s),
    )
        .cmp(&(
            OrderedF64(-rhs.terrain_min_clearance_m),
            OrderedF64(rhs.required_accel_ratio),
            OrderedF64(rhs.burn_time_s),
        ))
}

pub(super) fn command_throttle_for_applied_throttle(
    applied_throttle_frac: f64,
    min_throttle_frac: f64,
) -> f64 {
    let applied = applied_throttle_frac.clamp(0.0, 1.0);
    if applied <= 0.0 {
        return 0.0;
    }
    let min_throttle = min_throttle_frac.clamp(0.0, 1.0);
    if min_throttle >= 1.0 {
        return 1.0;
    }
    if applied <= min_throttle {
        return 1e-6;
    }
    ((applied - min_throttle) / (1.0 - min_throttle)).clamp(0.0, 1.0)
}

pub(super) fn projected_target_dx_after_lateral_brake(
    dx_m: f64,
    vx_mps: f64,
    brake_accel_mps2: f64,
    time_s: f64,
) -> f64 {
    if vx_mps.abs() <= 1e-6 || time_s <= 0.0 {
        return dx_m;
    }
    let accel = brake_accel_mps2.max(0.0);
    if accel <= 1e-6 {
        return dx_m - (vx_mps * time_s);
    }

    let stop_time_s = vx_mps.abs() / accel;
    if stop_time_s <= time_s {
        let stop_distance_m = (vx_mps * vx_mps) / (2.0 * accel);
        dx_m - (vx_mps.signum() * stop_distance_m)
    } else {
        let displacement_m = (vx_mps * time_s) - (0.5 * vx_mps.signum() * accel * time_s * time_s);
        dx_m - displacement_m
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OrderedF64(pub(super) f64);

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn estimate_target_y_projection(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    current_y_m: f64,
    gravity_mps2: f64,
) -> BallisticProjection {
    let target_y_cross = current_y_m + dy_m;
    let apex = ballistic_apex_from_state(current_y_m, vy_up_mps, gravity_mps2);
    if dy_m > 0.0 && (apex.time_to_apex_s <= 0.0 || apex.apex_y_m <= target_y_cross) {
        return BallisticProjection {
            projected_dx_m: dx_m - (vx_mps * apex.time_to_apex_s),
            time_to_cross_s: apex.time_to_apex_s.max(0.0),
            has_target_y_solution: false,
        };
    }

    if let Some(time_to_cross_s) = time_to_target_y_crossing(dy_m, vy_up_mps, gravity_mps2, true) {
        return BallisticProjection {
            projected_dx_m: dx_m - (vx_mps * time_to_cross_s),
            time_to_cross_s,
            has_target_y_solution: true,
        };
    }
    BallisticProjection {
        projected_dx_m: dx_m - (vx_mps * apex.time_to_apex_s),
        time_to_cross_s: apex.time_to_apex_s.max(0.0),
        has_target_y_solution: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BallisticApex {
    pub(super) time_to_apex_s: f64,
    pub(super) apex_y_m: f64,
}

pub(super) fn ballistic_apex_from_state(
    current_y_m: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
) -> BallisticApex {
    let time_to_apex_s = if gravity_mps2 <= 1e-6 {
        0.0
    } else {
        vy_up_mps / gravity_mps2
    };
    let apex_y_m = current_y_m + (vy_up_mps * time_to_apex_s)
        - (0.5 * gravity_mps2 * time_to_apex_s * time_to_apex_s);
    BallisticApex {
        time_to_apex_s,
        apex_y_m,
    }
}

pub(super) fn time_to_target_y_crossing(
    dy_m: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
    prefer_descending: bool,
) -> Option<f64> {
    let eps = 1e-6;
    if gravity_mps2 <= eps {
        if vy_up_mps.abs() <= eps {
            return if dy_m.abs() <= eps { Some(0.0) } else { None };
        }
        let t_lin = dy_m / vy_up_mps;
        return (t_lin >= 0.0).then_some(t_lin);
    }
    let disc = (vy_up_mps * vy_up_mps) - (2.0 * gravity_mps2 * dy_m);
    if disc < -eps {
        return None;
    }
    let sqrt_disc = disc.max(0.0).sqrt();
    let mut roots = [
        (vy_up_mps - sqrt_disc) / gravity_mps2,
        (vy_up_mps + sqrt_disc) / gravity_mps2,
    ];
    roots.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap());
    let positive: Vec<f64> = roots.into_iter().filter(|root| *root >= 0.0).collect();
    if positive.is_empty() {
        return None;
    }
    if prefer_descending && positive.len() >= 2 && dy_m >= 0.0 {
        return positive.last().copied();
    }
    positive
        .iter()
        .copied()
        .find(|root| *root > 1e-4)
        .or_else(|| positive.first().copied())
}

pub(super) fn estimate_ground_time_to_impact(
    height_to_target_m: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
) -> f64 {
    let altitude = height_to_target_m.max(0.0);
    let disc = (vy_up_mps * vy_up_mps) + (2.0 * gravity_mps2 * altitude);
    if disc <= 0.0 || gravity_mps2 <= 1e-6 {
        return 0.0;
    }
    let sqrt_disc = disc.sqrt();
    ((vy_up_mps + sqrt_disc) / gravity_mps2).max(0.0)
}
