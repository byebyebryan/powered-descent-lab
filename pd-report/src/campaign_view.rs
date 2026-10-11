//! Shared, strategy-independent display vocabulary for saved campaign attempts.
use pd_core::{BoundedRunStopCauseV1, PhysicalOutcome};
use serde::{Deserialize, Serialize};

use crate::campaign::{CampaignRowV1, escape, verified};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPresentationV1 {
    pub schema_version: u32,
    pub preview_href: String,
    pub caption: String,
    #[serde(default)]
    pub target_horizontal_offset_m: Option<f64>,
    #[serde(default)]
    pub height_above_upright_touchdown_m: Option<f64>,
    #[serde(default)]
    pub endpoint_speed_mps: Option<f64>,
}

impl CampaignPresentationV1 {
    pub(crate) fn validate(&self) -> bool {
        self.schema_version == 1
            && safe_asset_href(&self.preview_href)
            && [
                self.target_horizontal_offset_m,
                self.height_above_upright_touchdown_m,
                self.endpoint_speed_mps,
            ]
            .into_iter()
            .flatten()
            .all(f64::is_finite)
    }
}

fn safe_asset_href(href: &str) -> bool {
    !href.is_empty()
        && href.trim() == href
        && !href.starts_with('/')
        && !href.contains([':', '\\', '?', '#'])
        && !href.chars().any(char::is_control)
        && href.ends_with(".svg")
        && href
            .split('/')
            .all(|part| !part.is_empty() && part != ".." && part != ".")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisplayOutcome {
    Landed,
    Crashed,
    TimedOut,
    Stopped,
    NotLaunched,
    OffTarget,
    Unsupported,
    Failed,
    Unverified,
}

impl DisplayOutcome {
    pub const ALL: [Self; 9] = [
        Self::Landed,
        Self::Crashed,
        Self::TimedOut,
        Self::Stopped,
        Self::NotLaunched,
        Self::OffTarget,
        Self::Unsupported,
        Self::Failed,
        Self::Unverified,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Landed => "landed",
            Self::Crashed => "crashed",
            Self::TimedOut => "timed-out",
            Self::Stopped => "stopped",
            Self::NotLaunched => "not-launched",
            Self::OffTarget => "off-target",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
            Self::Unverified => "unverified",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Landed => "Verified landing",
            Self::Crashed => "Crashed",
            Self::TimedOut => "Timed out",
            Self::Stopped => "Stopped airborne",
            Self::NotLaunched => "Stopped before departure",
            Self::OffTarget => "Landed off target",
            Self::Unsupported => "Unsupported",
            Self::Failed => "Execution / capture failure",
            Self::Unverified => "Unverified outcome",
        }
    }
}

pub fn outcome(row: &CampaignRowV1, campaign_integrity: bool) -> DisplayOutcome {
    if row.disposition == "unsupported" {
        return DisplayOutcome::Unsupported;
    }
    if row.disposition != "executed" {
        return DisplayOutcome::Failed;
    }
    if !campaign_integrity
        || !row.execution_valid
        || !row.integrity_passed
        || !row.physical_replay_verified
        || !row.complete_replay_verified
    {
        return DisplayOutcome::Unverified;
    }
    if verified(row) {
        return DisplayOutcome::Landed;
    }
    match row.physical_outcome {
        Some(PhysicalOutcome::Crashed) => DisplayOutcome::Crashed,
        Some(PhysicalOutcome::TimedOut) => DisplayOutcome::TimedOut,
        Some(PhysicalOutcome::Flying) if row.sim_time_s == Some(0.0) => DisplayOutcome::NotLaunched,
        Some(PhysicalOutcome::Flying) => DisplayOutcome::Stopped,
        Some(PhysicalOutcome::LandedOffTarget) => DisplayOutcome::OffTarget,
        _ => DisplayOutcome::Unverified,
    }
}

pub fn stop_label(row: &CampaignRowV1) -> &'static str {
    match row.stop {
        Some(BoundedRunStopCauseV1::MissionTerminal) => "Mission ended",
        Some(BoundedRunStopCauseV1::ScenarioHorizonReached) => "Mission time limit",
        Some(BoundedRunStopCauseV1::CoverageExhausted) => "Saved command program ended",
        Some(BoundedRunStopCauseV1::HardDeadlineReached) => "Finite execution deadline",
        Some(BoundedRunStopCauseV1::SafetyRejected) => "Safety / solver stop",
        Some(BoundedRunStopCauseV1::ExecutionInvalid) => "Invalid execution",
        None => "No recorded flight",
    }
}

pub fn proof_label(row: &CampaignRowV1, integrity: bool) -> &'static str {
    if row.disposition != "executed" {
        "No flight proof"
    } else if integrity
        && row.execution_valid
        && row.integrity_passed
        && row.physical_replay_verified
        && row.complete_replay_verified
    {
        "Integrity and replay passed"
    } else {
        "Verification incomplete or failed"
    }
}

pub fn render_mission_cell(row: &CampaignRowV1) -> String {
    let label = format!(
        "Mission {} · attempt {}",
        row.mission_index, row.attempt_index
    );
    let link = row.report_href.as_ref().map_or_else(
        || escape(&label),
        |href| {
            format!(
                "<a class=\"mission-link\" href=\"{}\">{}</a>",
                escape(href),
                escape(&label)
            )
        },
    );
    let preview = row.presentation.as_ref().map_or_else(|| {
        "<span class=\"row-note muted\">Preview unavailable in this saved projection</span>".into()
    }, |view| {
        let image = format!("<img data-preview-src=\"{}\" width=\"156\" height=\"92\" alt=\"{}\" decoding=\"async\">", escape(&view.preview_href), escape(&view.caption));
        let plot = row.report_href.as_ref().map_or_else(|| format!("<span class=\"run-preview\">{image}</span>"), |href| format!("<a class=\"run-preview\" href=\"{}\" aria-label=\"Open flight for {}\">{image}</a>", escape(href), escape(&label)));
        format!("{plot}<span class=\"row-note muted preview-caption\">{}</span>", escape(&view.caption))
    });
    format!("<div class=\"preview-cell\">{link}{preview}</div>")
}

pub fn render_outcome_cell(row: &CampaignRowV1, integrity: bool) -> String {
    let status = outcome(row, integrity);
    let note = row.detail.as_ref().map_or_else(String::new, |value| {
        format!("<span class=\"row-note\">{}</span>", escape(value))
    });
    format!(
        "<span class=\"outcome-badge outcome-{}\">{}</span><span class=\"row-note\">{}</span>{note}",
        status.key(),
        status.label(),
        stop_label(row)
    )
}

pub fn render_evidence_cell(row: &CampaignRowV1, integrity: bool) -> String {
    format!(
        "<span class=\"proof-status\">{}</span><details class=\"row-evidence\"><summary>Identity and checks</summary><dl><dt>Physical input</dt><dd><code>{}</code></dd><dt>Execution valid</dt><dd>{}</dd><dt>Capture integrity</dt><dd>{}</dd><dt>Physical replay</dt><dd>{}</dd><dt>Complete replay</dt><dd>{}</dd><dt>Application wall time</dt><dd>{}</dd></dl></details>",
        proof_label(row, integrity),
        escape(&row.physical_sha256),
        row.execution_valid,
        row.integrity_passed,
        row.physical_replay_verified,
        row.complete_replay_verified,
        row.wall_time_us.map_or_else(
            || "Unavailable".into(),
            |n| format!("{:.2} ms", n as f64 / 1000.0)
        )
    )
}

/// Mutually exclusive scheduled-attempt partition; proof is reported separately.
pub fn render_outcome_bar(label: &str, rows: &[(&CampaignRowV1, bool)], approach: usize) -> String {
    let total = rows.len();
    let counts = DisplayOutcome::ALL.map(|status| {
        rows.iter()
            .filter(|(row, integrity)| outcome(row, *integrity) == status)
            .count()
    });
    let mut segments = String::new();
    let mut legend = String::new();
    for (status, count) in DisplayOutcome::ALL.into_iter().zip(counts) {
        if count == 0 {
            continue;
        }
        let pct = 100.0 * count as f64 / total.max(1) as f64;
        let title = format!("{}: {count}/{total} ({pct:.1}%)", status.label());
        segments.push_str(&format!(
            "<span class=\"bar-{}\" style=\"width:{pct:.5}%\" title=\"{}\"></span>",
            status.key(),
            escape(&title)
        ));
        legend.push_str(&format!("<button type=\"button\" data-review-outcome=\"{}\" data-approach=\"{approach}\"><span class=\"legend-dot bar-{}\"></span>{} <strong>{count}</strong> <span class=\"muted\">{pct:.1}%</span></button>", status.key(), status.key(), status.label()));
    }
    let landed = counts[0];
    let executed = rows
        .iter()
        .filter(|(r, _)| r.disposition == "executed")
        .count();
    let proof = rows
        .iter()
        .filter(|(r, integrity)| {
            *integrity
                && r.disposition == "executed"
                && r.execution_valid
                && r.integrity_passed
                && r.physical_replay_verified
                && r.complete_replay_verified
        })
        .count();
    format!(
        "<article class=\"outcome-card\" data-outcome-approach=\"{approach}\"><div class=\"outcome-card-head\"><h3>{}</h3><strong>{landed}/{total} <span class=\"muted\">verified landings · {:.1}%</span></strong></div><div class=\"outcome-bar\" role=\"img\" aria-label=\"{}\">{segments}</div><div class=\"outcome-legend\">{legend}</div><p class=\"proof-note\">Saved integrity and complete replay: {proof}/{executed} executed attempts. All {total} scheduled attempts remain in the outcome denominator.</p></article>",
        escape(label),
        100.0 * landed as f64 / total.max(1) as f64,
        escape(&format!("{label}: {landed}/{total} verified landings"))
    )
}

pub const STYLE: &str = include_str!("campaign_view.css");
pub const SCRIPT: &str = include_str!("campaign_view.js");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_assets_are_relative_bounded_and_finite() {
        let mut view = CampaignPresentationV1 {
            schema_version: 1,
            preview_href: "previews/flight.svg".into(),
            caption: "Saved flight".into(),
            target_horizontal_offset_m: None,
            height_above_upright_touchdown_m: None,
            endpoint_speed_mps: None,
        };
        assert!(view.validate());
        for href in [
            "javascript:x.svg",
            "//host/x.svg",
            "../x.svg",
            "bad\\x.svg",
            "x.svg#fragment",
            "x.svg\n",
        ] {
            view.preview_href = href.into();
            assert!(!view.validate(), "{href}");
        }
        view.preview_href = "flight.svg".into();
        view.endpoint_speed_mps = Some(f64::NAN);
        assert!(!view.validate());
    }

    fn row() -> CampaignRowV1 {
        CampaignRowV1 {
            attempt_index: 0,
            mission_index: 0,
            physical_sha256: "1".repeat(64),
            terrain_family: "test".into(),
            direction: 1,
            terrain_stratum: 0,
            disposition: "executed".into(),
            detail: None,
            stop: Some(BoundedRunStopCauseV1::MissionTerminal),
            physical_outcome: Some(PhysicalOutcome::LandedOnTarget),
            mission_outcome: Some(pd_core::MissionOutcome::Success),
            execution_valid: true,
            integrity_passed: true,
            physical_replay_verified: true,
            complete_replay_verified: true,
            verified_target_landing: true,
            sim_time_s: Some(10.0),
            fuel_kg: Some(1.0),
            wall_time_us: None,
            report_href: Some("attempt/report.html".into()),
            presentation: None,
        }
    }

    #[test]
    fn physical_misses_unsupported_and_missing_proof_cannot_become_landings() {
        let mut value = row();
        assert_eq!(outcome(&value, true), DisplayOutcome::Landed);
        assert_eq!(outcome(&value, false), DisplayOutcome::Unverified);
        value.complete_replay_verified = false;
        assert_eq!(outcome(&value, true), DisplayOutcome::Unverified);
        value.complete_replay_verified = true;
        value.verified_target_landing = false;
        for (physical, expected) in [
            (PhysicalOutcome::Crashed, DisplayOutcome::Crashed),
            (PhysicalOutcome::TimedOut, DisplayOutcome::TimedOut),
            (PhysicalOutcome::Flying, DisplayOutcome::Stopped),
            (PhysicalOutcome::LandedOffTarget, DisplayOutcome::OffTarget),
        ] {
            value.physical_outcome = Some(physical);
            assert_eq!(outcome(&value, true), expected);
        }
        value.physical_outcome = Some(PhysicalOutcome::Flying);
        value.sim_time_s = Some(0.0);
        assert_eq!(outcome(&value, true), DisplayOutcome::NotLaunched);
        value.disposition = "unsupported".into();
        assert_eq!(outcome(&value, true), DisplayOutcome::Unsupported);
        value.disposition = "initialization_failed".into();
        assert_eq!(outcome(&value, true), DisplayOutcome::Failed);
    }

    #[test]
    fn old_projections_keep_their_encoding_and_new_previews_are_escaped_and_deferred() {
        let mut value = row();
        let encoded = serde_json::to_value(&value).unwrap();
        assert!(encoded.get("presentation").is_none());
        let restored: CampaignRowV1 = serde_json::from_value(encoded.clone()).unwrap();
        assert!(restored.presentation.is_none());
        assert_eq!(serde_json::to_value(restored).unwrap(), encoded);
        value.presentation = Some(CampaignPresentationV1 {
            schema_version: 1,
            preview_href: "previews/test.svg".into(),
            caption: "Recorded <flight>".into(),
            target_horizontal_offset_m: None,
            height_above_upright_touchdown_m: None,
            endpoint_speed_mps: None,
        });
        let html = render_mission_cell(&value);
        assert!(html.contains("class=\"run-preview\""));
        assert!(html.contains("data-preview-src=\"previews/test.svg\""));
        assert!(!html.contains("<img src="));
        assert!(html.contains("Recorded &lt;flight&gt;"));
    }
}
