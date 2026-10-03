//! Display-only evidence. The evaluator owns validation and flight semantics.

use pd_core::Vec2;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlightPoint {
    pub physics_step: u64,
    pub time_s: f64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    InitialNominal,
    LocalCorrection,
    ReplannedNominal,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Correction {
    pub number: usize,
    pub reason: String,
    pub after_handoff: String,
    /// An audited hypothetical conflict, never an executed path point.
    pub not_flown_conflict: Option<FlightPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlownSegment {
    pub kind: SegmentKind,
    /// Actual samples between exact, retained entry and end snapshots.
    pub points: Vec<FlightPoint>,
    pub correction: Option<Correction>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReportLink {
    pub label: String,
    pub href: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlightReport {
    pub title: String,
    pub case_id: String,
    pub outcome: String,
    pub landed: bool,
    pub correction_count: u32,
    pub elapsed_s: f64,
    pub terrain: Vec<Vec2>,
    pub source_pad: Vec2,
    pub target_pad: Vec2,
    pub start: FlightPoint,
    pub finish: FlightPoint,
    pub segments: Vec<FlownSegment>,
    pub index_href: Option<String>,
    pub capture_label: String,
    pub policy_version: u32,
    pub diagnostics: Vec<(String, String)>,
    pub source_links: Vec<ReportLink>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CaseCard {
    pub case_id: String,
    pub title: String,
    pub group: String,
    pub outcome: String,
    pub correction_count: u32,
    /// Unsupported preflight results deliberately have no flight report.
    pub href: Option<String>,
    pub inspect: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SuiteReport {
    pub title: String,
    pub capture_label: String,
    pub policy_version: u32,
    pub cases: Vec<CaseCard>,
    pub diagnostics: Vec<(String, String)>,
    pub source_links: Vec<ReportLink>,
}
