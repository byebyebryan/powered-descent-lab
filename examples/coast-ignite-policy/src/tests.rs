use super::*;

fn tasks() -> Vec<serde_json::Value> {
    let anchor: ScenarioSpec = serde_json::from_str(include_str!(
        "../../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .unwrap();
    (0..256)
        .filter_map(|i| {
            let p = generator::proposal(&anchor, 919, "software-test", i).unwrap();
            (p["admitted"] == true).then_some(p)
        })
        .take(3)
        .collect()
}

#[test]
fn generator_matches_discrete_ballistic_geometry_and_is_deterministic() {
    let proposals = tasks();
    assert_eq!(proposals.len(), 3);
    for p in proposals {
        let s: ScenarioSpec = serde_json::from_value(p["scenario"].clone()).unwrap();
        let ticks = p["audit"]["nominal_coast_ticks"].as_u64().unwrap()
            + p["audit"]["ballistic_ticks"].as_u64().unwrap();
        let (mut y, mut x, mut vy) = (
            s.initial_state.position_m.y,
            s.initial_state.position_m.x,
            s.initial_state.velocity_mps.y,
        );
        for _ in 0..ticks {
            vy -= 9.81 / 120.0;
            y += vy / 120.0;
            x += s.initial_state.velocity_mps.x / 120.0;
        }
        assert!((y - 5.0).abs() < 1e-8);
        assert!(x.abs() < 1e-8);
        assert_eq!(p["audit"]["native_forecast_steps"], 0);
    }
    assert_eq!(tasks(), tasks());
}

#[test]
fn summary_capture_same_tick_ignition_and_never_have_exact_native_parity() {
    Python::attach(|py| {
        let mut saw_odd_contact = false;
        for p in tasks() {
            let end = p["reference_end"].as_u64().unwrap();
            let nominal = p["audit"]["nominal_coast_ticks"].as_u64().unwrap();
            for ignition in [Some(0), Some(nominal), Some(nominal + 12), None] {
                let mut summary =
                    CoastEpisode::new(&p["scenario"].to_string(), end, false, None, None).unwrap();
                let mut capture =
                    CoastEpisode::new(&p["scenario"].to_string(), end, true, None, None).unwrap();
                loop {
                    let tick = summary.live().unwrap().observation().physics_step;
                    let action = u8::from(ignition.is_some_and(|i| tick >= i));
                    let a = summary.step(py, action).unwrap();
                    let b = capture.step(py, action).unwrap();
                    assert_eq!(a, b);
                    assert_eq!(
                        summary.live().unwrap().retained_record_counts(),
                        (0, 0, 0, 0)
                    );
                    if a.2 {
                        break;
                    }
                }
                assert!(summary.step(py, 0).is_err());
                let (a, info_a) = summary.finish().unwrap();
                let (b, info_b) = capture.finish().unwrap();
                assert_eq!(a.summary, b.summary);
                assert_eq!(info_a, info_b);
                saw_odd_contact |= a.summary.final_state.physics_step % 2 == 1;
                if ignition.is_none() {
                    let scenario: ScenarioSpec = decode(&p["scenario"].to_string()).unwrap();
                    assert_eq!(
                        a.summary.final_state.fuel_kg,
                        scenario.vehicle.initial_fuel_kg
                    );
                }
                if let Some(active) = capture.controller.feedback().and_then(|f| f.accepted) {
                    let tick = active.entry.physics_step;
                    let command = b
                        .agent_updates
                        .as_ref()
                        .unwrap()
                        .iter()
                        .find(|u| u.physics_step == tick)
                        .unwrap();
                    assert!(command.frame.command.throttle_frac > 0.0);
                    assert_eq!(tick, ignition.unwrap());
                }
                let execution = BoundedAgentRunArtifactsV1 {
                    descriptor: capture.descriptor,
                    episode: b.capture.unwrap(),
                    controller_updates: b.agent_updates.unwrap(),
                    performance: RunPerformanceStats {
                        wall_time_us: 0,
                        thread_cpu_time_us: None,
                    },
                };
                assert!(
                    pd_eval::verify_execution(&capture.scenario, &execution)
                        .unwrap()
                        .complete_episode_verified
                );
            }
        }
        assert!(
            saw_odd_contact,
            "exercise actual off-command-boundary contact"
        );
    });
}

#[test]
fn rejected_ignition_is_one_attempt_and_preserves_coast_until_native_end() {
    Python::attach(|py| {
        let p = tasks().remove(0);
        let mut s: ScenarioSpec = decode(&p["scenario"].to_string()).unwrap();
        s.vehicle.initial_fuel_kg = 5.0;
        let mut episode = CoastEpisode::new(
            &json(&s).unwrap(),
            p["reference_end"].as_u64().unwrap(),
            false,
            None,
            None,
        )
        .unwrap();
        let (_, reward, done, _) = episode.step(py, 1).unwrap();
        assert!(done);
        assert_eq!(reward, 0.0);
        let (result, info) = episode.finish().unwrap();
        assert!(result.summary.failure.is_none());
        assert_eq!(result.summary.final_state.fuel_kg, 5.0);
        assert_eq!(info["decisions"].as_array().unwrap().len(), 1);
        assert_eq!(info["feedback"]["constructions"], 1);
        assert!(info["feedback"]["accepted"].is_null());
        let diagnostic = episode.diagnostics.as_ref().unwrap();
        assert_eq!(diagnostic["admission"]["kind"], "no_plan");
        assert!(diagnostic["admission"]["reason"].is_string());
        assert_eq!(diagnostic["native_forecast_steps"], 0);
        assert_eq!(
            diagnostic["incoming_contact"]["geometry"]["classification"],
            diagnostic["incoming_contact"]["recorded"]["classification"]
        );
    });
}
