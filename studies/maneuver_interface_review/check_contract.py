#!/usr/bin/env python3
"""Bounded abstract contract checks; imports no flight or learning implementation."""

import argparse
from collections import deque
from dataclasses import asdict, dataclass, replace
from fractions import Fraction
import hashlib
import json
from pathlib import Path


EVENTS = (
    "keep", "start_a", "start_b", "changed_payload", "unsupported",
    "cancel", "stale", "terminal_before", "contact_after_one",
)
REJECTIONS = {"id_conflict", "unsupported", "busy", "stale"}
MUTATIONS = (
    "reset_on_duplicate", "drop_on_rejection", "resurrect_after_end",
    "restart_clock", "wait_for_gate", "hold_backend_command", "fake_completion",
)


@dataclass(frozen=True)
class State:
    phase: str = "coast"
    request: str | None = None
    tick: int = 0
    resets: int = 0
    backend_updates: int = 0
    commands: int = 0


@dataclass(frozen=True)
class Transition:
    after: State
    acknowledgement: str
    owner: str | None
    command_token: int | None
    advanced: int


def respond(state, event):
    """Reference admission rules, independent of command production."""
    if event in {"keep", "contact_after_one"}:
        return state, "keep"
    if event == "stale":
        return state, "stale"
    if event == "changed_payload":
        return state, "id_conflict" if state.request else "unsupported"
    if event in {"unsupported", "cancel"}:
        return state, "unsupported"
    request = event.removeprefix("start_")
    if state.phase == "active":
        return state, "already_active" if request == state.request else "busy"
    return replace(state, phase="active", request=request, resets=state.resets + 1), "accepted"


def advance(state, event, mutation=None, deadline=None):
    """One abstract 60 Hz command interval over a 120 Hz integer clock."""
    original = state
    if state.phase == "closed":
        if mutation == "resurrect_after_end" and event.startswith("start_"):
            state = replace(state, phase="coast", request=None)
        else:
            return Transition(state, "not_called", None, None, 0)
    if event == "terminal_before" or (deadline is not None and state.tick >= deadline):
        return Transition(replace(state, phase="closed"), "not_called", None, None, 0)
    state, acknowledgement = respond(state, event)
    if mutation == "reset_on_duplicate" and acknowledgement == "already_active":
        state = replace(state, resets=state.resets + 1, backend_updates=0)
    if mutation == "drop_on_rejection" and acknowledgement in REJECTIONS:
        state = replace(state, phase="coast", request=None)
    if mutation == "restart_clock" and acknowledgement == "accepted":
        state = replace(state, tick=0)
    owner = "terminal" if state.phase == "active" else "coast"
    if mutation == "wait_for_gate" and acknowledgement == "accepted":
        owner = "coast"
    updates = state.backend_updates + int(owner == "terminal")
    command_token = updates if owner == "terminal" else 0
    if mutation == "hold_backend_command" and owner == "terminal":
        command_token = min(updates, 1)
    advanced = 1 if event == "contact_after_one" else 2
    if deadline is not None:
        advanced = min(advanced, deadline - state.tick)
    state = replace(
        state, tick=state.tick + advanced, backend_updates=updates,
        commands=state.commands + 1,
    )
    if event == "contact_after_one" or (deadline is not None and state.tick == deadline):
        state = replace(state, phase="closed")
    if mutation == "fake_completion" and updates >= 2:
        state = replace(state, phase="closed")
    # Return the clock change explicitly, allowing the invariants to detect resets.
    return Transition(state, acknowledgement, owner, command_token, state.tick - original.tick)


def invariant(before, event, transition):
    after = transition.after
    if before.phase == "closed":
        if after != before or transition.owner is not None:
            return "terminal state is absorbing"
        return None
    if event == "terminal_before":
        if (after.tick, after.commands, after.backend_updates, transition.owner) != (
            before.tick, before.commands, before.backend_updates, None,
        ) or after.phase != "closed":
            return "native end precedes request and command callbacks"
        return None
    expected_advance = 1 if event == "contact_after_one" else 2
    if transition.advanced != expected_advance:
        return "requests preserve the absolute clock and consume physical time"
    if after.commands != before.commands + 1:
        return "exactly one owner emits each live command"
    if after.resets not in {0, 1}:
        return "one accepted landing creates one backend instance"
    if transition.acknowledgement in REJECTIONS | {"already_active", "keep"}:
        if after.resets != before.resets:
            return "keep rejection and duplicate requests preserve backend state"
        if before.phase == "active" and after.request != before.request:
            return "rejection preserves the active request"
    active = before.phase == "active" or transition.acknowledgement == "accepted"
    expected_owner = "terminal" if active else "coast"
    if transition.owner != expected_owner:
        return "activation grants immediate and continuous command authority"
    expected_updates = before.backend_updates + int(active)
    if after.backend_updates != expected_updates:
        return "the active backend updates on every command interval"
    if transition.command_token != (expected_updates if active else 0):
        return "holding a request does not hold a stale backend command"
    if (after.phase == "closed") != (event == "contact_after_one"):
        return "controller telemetry cannot manufacture native completion"
    return None


def explore(depth, mutation=None):
    queue = deque([(State(), ())])
    seen = {State()}
    edges = 0
    while queue:
        before, trace = queue.popleft()
        if len(trace) == depth:
            continue
        for event in EVENTS:
            transition = advance(before, event, mutation)
            edges += 1
            violation = invariant(before, event, transition)
            if violation:
                return {
                    "passed": False, "invariant": violation,
                    "trace": list(trace + (event,)), "before": asdict(before),
                    "transition": asdict(transition), "edges_checked": edges,
                }
            if transition.after not in seen:
                seen.add(transition.after)
                queue.append((transition.after, trace + (event,)))
    return {"passed": True, "states_checked": len(seen), "edges_checked": edges, "depth": depth}


def check_admission():
    active = advance(State(), "start_a").after
    cases = [
        (State(), "start_a", "accepted"),
        (active, "start_a", "already_active"),
        (active, "start_b", "busy"),
        (active, "changed_payload", "id_conflict"),
        (active, "cancel", "unsupported"),
        (State(), "unsupported", "unsupported"),
        (active, "stale", "stale"),
        (State(), "stale", "stale"),
    ]
    for state, event, expected in cases:
        actual = advance(state, event)
        if actual.acknowledgement != expected:
            raise AssertionError((event, actual.acknowledgement, expected))
    return {"passed": True, "cases": len(cases)}


def check_scheduling():
    cases = 0
    for deadline in range(32):
        for activation in (0, 12, 24, None):
            state = State()
            planner_ticks = []
            control_ticks = []
            if deadline == 0:
                state = advance(state, "keep", deadline=deadline).after
            while state.phase != "closed":
                event = "keep"
                control_ticks.append(state.tick)
                if state.tick % 12 == 0 and state.phase == "coast":
                    planner_ticks.append(state.tick)
                    if activation is not None and state.tick == activation:
                        event = "start_a"
                state = advance(state, event, deadline=deadline).after
            if state.tick != deadline or state.commands != (deadline + 1) // 2:
                raise AssertionError("native partial termination or cadence mismatch")
            expected_planner = [
                tick for tick in range(0, deadline, 12)
                if activation is None or tick <= activation
            ]
            if planner_ticks != expected_planner or control_ticks != list(range(0, deadline, 2)):
                raise AssertionError("planner and controller clocks must remain independent")
            expected_updates = (
                (deadline - activation + 1) // 2
                if activation is not None and activation < deadline else 0
            )
            if state.backend_updates != expected_updates:
                raise AssertionError("controller updates were held at planner frequency")
            cases += 1
    return {
        "passed": True, "cases": cases, "physics_hz": 120,
        "controller_hz": 60, "planner_hz": 10,
        "includes_odd_tick_endpoints": True,
        "planner_called_only_when_a_choice_exists": True,
    }


def partitions(length):
    for mask in range(1 << (length - 1)):
        cuts = [0] + [i for i in range(1, length) if mask & (1 << (i - 1))] + [length]
        yield list(zip(cuts, cuts[1:]))


def grouped_return(rewards, gamma, value, groups, mutation=None):
    result = value
    for start, end in reversed(groups):
        block = sum(gamma ** (i - start) * rewards[i] for i in range(start, end))
        if mutation == "drop_intermediate_rewards":
            block = gamma ** (end - start - 1) * rewards[end - 1]
        exponent = 1 if mutation == "unit_duration_discount" else end - start
        result = block + gamma ** exponent * result
    return result


def check_returns():
    cases = 0
    caught = set()
    for rewards in (
        tuple(map(Fraction, [0, 0, 0, 0, 0, 0, 0, 1])),
        tuple(map(Fraction, [1, -2, 0, 3, 0, -1, 2, -4])),
    ):
        for gamma in (Fraction(1), Fraction(9, 10)):
            for bootstrap in (Fraction(0), Fraction(3, 2)):
                expected = sum(gamma ** i * reward for i, reward in enumerate(rewards))
                expected += gamma ** len(rewards) * bootstrap
                for groups in partitions(len(rewards)):
                    if grouped_return(rewards, gamma, bootstrap, groups) != expected:
                        raise AssertionError("grouped return must preserve primitive-time accounting")
                    for mutation in ("unit_duration_discount", "drop_intermediate_rewards"):
                        if grouped_return(rewards, gamma, bootstrap, groups, mutation) != expected:
                            caught.add(mutation)
                    cases += 1
    if len(caught) != 2:
        raise AssertionError("return checks must reject both deliberate accounting faults")
    return {
        "passed": True, "cases": cases, "exact_rational_arithmetic": True,
        "terminal_and_nonterminal_bootstrap_examples": True,
        "negative_controls_detected": sorted(caught),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--depth", type=int, default=8)
    parser.add_argument("--output", type=Path, help="optional create-only JSON receipt")
    args = parser.parse_args()
    if not 3 <= args.depth <= 12:
        parser.error("depth must be in [3, 12]")
    model = explore(args.depth)
    if not model["passed"]:
        raise AssertionError(model)
    negative_controls = {mutation: explore(args.depth, mutation) for mutation in MUTATIONS}
    if any(result["passed"] for result in negative_controls.values()):
        raise AssertionError("every deliberate lifecycle fault must be detected")
    result = {
        "schema_version": 1, "passed": True,
        "scope": "bounded abstract design model and accounting examples only",
        "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "lifecycle": model, "admission": check_admission(),
        "scheduling": check_scheduling(), "returns": check_returns(),
        "lifecycle_negative_controls": negative_controls,
        "native_flights": 0, "optimizer_updates": 0,
        "production_adapter_validated": False,
        "controller_capability_qualified": False,
    }
    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output is not None:
        # Fail before creating parents or changing an existing evidence destination.
        if not args.output.parent.is_dir():
            parser.error("receipt parent must already exist")
        with args.output.open("x") as stream:
            stream.write(payload)
    summary = dict(result)
    summary["lifecycle_negative_controls"] = {
        name: {"invariant": value["invariant"], "trace": value["trace"]}
        for name, value in negative_controls.items()
    }
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
