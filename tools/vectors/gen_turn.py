#!/usr/bin/env python3
"""Golden vectors for aviation.performance.turn."""
import json
import math
import sys
from pathlib import Path

OLD_SRC = "Independent Python implementation of the turn, gradient, glide, and pivotal-altitude relations in FAA-H-8083-3C and FAA-H-8083-16B (tools/vectors/gen_aviation.py)"
OLD_VER = "FAA-H-8083-3C (2021)"
SRC = "Coordinated-turn relations evaluated in Python (tools/vectors/gen_turn.py)"
FAA = "FAA Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), chapter 5, figure 5-56"
KT = 1852.0 / 3600.0
FT = 0.3048
G0 = 9.80665


def vector(i, inputs, expected, source=SRC, version="2026", rel=1e-9, abs_tol=1e-6):
    expected = dict(expected)
    expected.setdefault("ok", True)
    tolerance = {
        key: {"abs": abs_tol, "rel": rel}
        for key, value in expected.items()
        if isinstance(value, (int, float)) and not isinstance(value, bool)
    }
    return {
        "id": f"v{i:03d}",
        "input": inputs,
        "expect": expected,
        "source": source,
        "sourceVersion": version,
        "tolerance": tolerance,
    }


def turn(knots, bank):
    speed = knots * KT
    phi = math.radians(bank)
    rate = G0 * math.tan(phi) / speed
    return {
        "result.bank.value": bank,
        "result.turn_rate.value": math.degrees(rate),
        "result.radius.value": speed * speed / (G0 * math.tan(phi)) / FT,
        "result.load_factor": 1.0 / math.cos(phi),
    }


def vectors():
    # Preserve the six originally published vectors byte for byte.
    out = [
        vector(1, {"tas": "100 kt"}, {"result.bank.value": 15.35884675474713, "result.radius.value": 3223.4793810825695, "result.load_factor": 1.037036844012689}, OLD_SRC, OLD_VER),
        vector(2, {"tas": "150 kt", "bank": "30 deg"}, {"result.bank.value": 29.999999999999996, "result.radius.value": 3450.519375232292, "result.load_factor": 1.1547005383792515}, OLD_SRC, OLD_VER),
        vector(3, {"tas": "90 kt", "bank": "45 deg"}, {"result.bank.value": 45.0, "result.radius.value": 717.176984448378, "result.load_factor": 1.414213562373095}, OLD_SRC, OLD_VER),
        vector(4, {"tas": "250 kt", "bank": "25 deg"}, {"result.bank.value": 25.0, "result.radius.value": 11867.214555399089, "result.load_factor": 1.1033779189624917}, OLD_SRC, OLD_VER),
        vector(5, {"tas": "60 kt", "bank": "60 deg"}, {"result.bank.value": 59.99999999999999, "result.radius.value": 184.02770001238892, "result.load_factor": 1.9999999999999996}, OLD_SRC, OLD_VER),
        vector(6, {"tas": "120 kt"}, {"result.bank.value": 18.24263059046449, "result.radius.value": 3868.175257299084, "result.load_factor": 1.0529204142808273}, OLD_SRC, OLD_VER),
    ]
    out.append(vector(7, {"tas": "120 kt", "bank": "30 deg"},
                      {"result.turn_rate.value": 5.25}, FAA, "FAA-H-8083-25C (2023)", abs_tol=0.005))
    for knots, bank in [(60, 15), (100, 20), (120, 30), (240, 30), (90, 45)]:
        out.append(vector(len(out) + 1,
                          {"tas": f"{knots} kt", "bank": f"{bank} deg"}, turn(knots, bank)))
    for knots, rate in [(100, 3), (160, 2.5)]:
        bank = math.degrees(math.atan(knots * KT * math.radians(rate) / G0))
        out.append(vector(len(out) + 1,
                          {"tas": f"{knots} kt", "turn_rate": f"{rate} deg/s"},
                          {"result.bank.value": bank}))
    bank, rate = 30, 5.25
    solved_knots = G0 * math.tan(math.radians(bank)) / math.radians(rate) / KT
    out.append(vector(15, {"bank": "30 deg", "turn_rate": "5.25 deg/s"},
                      {"result.tas.value": solved_knots}))
    out.append(vector(16, {"tas": "100 kt", "heading_change": "180 deg"},
                      {"result.turn_time.value": 60.0}))
    load = 1.0 / math.cos(math.radians(60))
    out.append(vector(17, {"tas": "100 kt", "bank": "60 deg", "stall_speed": "50 kt", "load_limit": 3.8}, {
        "result.stall_speed_in_turn.value": 50.0 * math.sqrt(load),
        "result.max_bank.value": math.degrees(math.acos(1.0 / 3.8)),
    }))
    out.extend([
        vector(18, {"tas": "0 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/tas"}, "The tool's stated domain"),
        vector(19, {"tas": "100 kt", "bank": "0 deg"},
               {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/bank"}, "The tool's stated domain"),
        vector(20, {"tas": "100 kt", "bank": "30 deg", "turn_rate": "3 deg/s"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/turn_rate"}, "The tool's stated domain"),
    ])
    assert len(out) == 20
    return out


def main():
    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "aviation.performance.turn.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors()))


if __name__ == "__main__":
    main()
