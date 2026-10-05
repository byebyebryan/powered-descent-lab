//! Small live contact and source-input helpers shared by planner flight execution.

use anyhow::{Context, Result};
use pd_core::{FlightProgramUpdateV1, RunContext};
use pd_plan::ballistic::PadInputV2;

use crate::AirborneDirectAuditV1;

pub(crate) fn has_actual_terrain_conflict(audit: &AirborneDirectAuditV1) -> bool {
    audit.clearance_scan.first_violation.is_some()
        || audit
            .first_contact
            .as_ref()
            .is_some_and(contact_is_actual_away_terrain_contact)
}

fn contact_is_actual_away_terrain_contact(contact: &crate::TerminalContactAuditEvidence) -> bool {
    contact_has_away_geometric_contact(
        &contact.classification,
        contact.core_matches_predicate_mirror,
        contact.state.touchdown_pad_contains_both_feet,
        contact.state.minimum_hull_clearance_m,
        [
            contact.state.touchdown_feet[0].signed_clearance_m,
            contact.state.touchdown_feet[1].signed_clearance_m,
        ],
    )
}

fn contact_has_away_geometric_contact(
    classification: &str,
    core_matches_predicate_mirror: bool,
    touchdown_feet_within_target: bool,
    minimum_hull_clearance_m: f64,
    foot_signed_clearance_m: [f64; 2],
) -> bool {
    classification != "stable_touchdown_on_target"
        && core_matches_predicate_mirror
        && !touchdown_feet_within_target
        && (minimum_hull_clearance_m <= 0.0
            || foot_signed_clearance_m
                .iter()
                .any(|clearance| *clearance <= 0.0))
}

pub(crate) fn phase_at_tick(updates: &[FlightProgramUpdateV1], tick: u64) -> String {
    updates
        .iter()
        .take_while(|update| update.physics_step < tick)
        .last()
        .map(|update| update.phase.clone())
        .unwrap_or_else(|| "source_prefix".into())
}

pub(crate) fn source_pad_input(context: &RunContext, source_pad_id: &str) -> Result<PadInputV2> {
    let pad = context
        .world
        .landing_pad(source_pad_id)
        .context("source pad missing from actual context")?;
    Ok(PadInputV2 {
        center_x_m: pad.center_x_m,
        surface_y_m: pad.surface_y_m,
        width_m: pad.width_m,
    })
}
