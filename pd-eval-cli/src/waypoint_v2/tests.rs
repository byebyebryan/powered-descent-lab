use super::*;
use crate::WaypointDirectNominalDirectGenerationRequest;

/// Explicitly opted-in numerical regression against a validated retained
/// capture. Runs the current planner without creating/publishing artifacts;
/// only the three nondeterministic flight wall-time fields are excluded.
#[test]
#[ignore = "set PD_V2_PARITY_CAPTURE to a retained accepted V2 capture"]
fn retained_capture_numerical_parity() {
    let capture = std::path::PathBuf::from(
        std::env::var_os("PD_V2_PARITY_CAPTURE").expect("PD_V2_PARITY_CAPTURE is required"),
    );
    // Cargo runs library tests from the crate directory. Resolve relative
    // evidence roots from the workspace; absolute paths remain absolute.
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(capture);
    let acceptance = crate::waypoint_v2_acceptance::check_waypoint_v2_acceptance(&capture)
        .expect("validate retained capture before using it as a baseline");
    assert!(acceptance.passed, "retained baseline must be accepted");
    let batch = crate::waypoint_v2_pack::validated_waypoint_v2_batch(&capture).unwrap();
    let (inputs, identity) = crate::waypoint_v2_pack::check_default_planner_v2_binding().unwrap();
    assert_eq!(batch.input_identity, identity);
    assert_eq!(batch.cases.len(), inputs.len());

    for input in inputs {
        let case = batch
            .cases
            .iter()
            .find(|case| case.case_id == input.case_id)
            .unwrap();
        let mut expected: WaypointV2FlightResult =
            serde_json::from_slice(&std::fs::read(capture.join(&case.flight_path)).unwrap())
                .unwrap();
        let request = WaypointDirectNominalDirectGenerationRequest {
            probe_id: input.scenario.id.clone(),
            scenario: input.scenario,
            source_pad_id: input.source_pad_id,
            target_pad_id: input.target_pad_id,
            policy: crate::WaypointDirectNominalDirectGenerationPolicyV1::default(),
        };
        let mut actual = run_waypoint_v2_flight(&request, &expected.policy).unwrap();
        actual.timings = WaypointV2Timings::default();
        expected.timings = WaypointV2Timings::default();
        let difference = crate::first_json_difference(
            &serde_json::to_value(actual).unwrap(),
            &serde_json::to_value(expected).unwrap(),
            "$",
        );
        assert!(difference.is_none(), "{}: {difference:?}", case.case_id);
        eprintln!("{}: exact numerical parity", case.case_id);
    }
}
