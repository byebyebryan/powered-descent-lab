//! File frontdoor for the solver-owned counterfactual handoff diagnostic.
use crate::evidence_io::{reserve_output_root, sha256_bytes, write_json_create_only};
use anyhow::{Result, ensure};
pub use pd_planner::waypoint_v2::handoff_probe::{ProbeArm, ProbeSpec};
use pd_planner::{
    DiagnosticRecorder, WaypointDirectNominalDirectGenerationPolicyV1,
    WaypointDirectNominalDirectGenerationRequest, WaypointV2FlightResult,
};
use std::{fs, path::Path};
struct Recorder<'a>(&'a Path);
impl DiagnosticRecorder for Recorder<'_> {
    fn write_json<T: serde::Serialize + ?Sized>(&mut self, name: &str, value: &T) -> Result<()> {
        write_json_create_only(&self.0.join(name), value)
    }
}
pub fn run(
    scenario_path: &Path,
    flight_path: &Path,
    spec_path: &Path,
    output: &Path,
) -> Result<serde_json::Value> {
    let spec: ProbeSpec = serde_json::from_slice(&fs::read(spec_path)?)?;
    pd_planner::waypoint_v2::handoff_probe::validate_spec(&spec)?;
    let scenario_bytes = fs::read(scenario_path)?;
    let flight_bytes = fs::read(flight_path)?;
    ensure!(
        sha256_bytes(&scenario_bytes)? == spec.scenario_sha256
            && sha256_bytes(&flight_bytes)? == spec.flight_sha256,
        "changed diagnostic source artifacts"
    );
    let scenario: pd_core::ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
    let request = WaypointDirectNominalDirectGenerationRequest {
        probe_id: scenario.id.clone(),
        scenario,
        source_pad_id: "pad_source".into(),
        target_pad_id: "pad_main".into(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    };
    pd_planner::validate_waypoint_direct_nominal_direct_generation_request(&request)?;
    let baseline: WaypointV2FlightResult = serde_json::from_slice(&flight_bytes)?;
    pd_planner::waypoint_v2::handoff_probe::validate_source(&request, &baseline)?;
    reserve_output_root(output)?;
    pd_planner::waypoint_v2::handoff_probe::run_resolved(
        &request,
        &baseline,
        &spec,
        &mut Recorder(output),
    )
}
