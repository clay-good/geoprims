#!/usr/bin/env python3
"""Golden vectors for aviation.loading.burn-rate.

The published cases come from FAA worked examples. The remaining cases use a
small, separate evaluation of fuel / time and usable fuel / rate so unit and
boundary behavior are exercised outside the Rust implementation.
"""
import json
import sys
from pathlib import Path

GAL_L = 3.785411784
SRC = "Fuel / time and usable fuel / rate evaluated in Python (tools/vectors/gen_burn_rate.py)"
SPEC = "Arithmetic of the definitions (fuel used / time; usable fuel / rate), the add-aviation-suite burn-rate scenario first"
PHAK = "FAA Pilot's Handbook of Aeronautical Knowledge, FAA-H-8083-25C, chapter 16, Fuel Consumption"
CURRICULUM = "FAA Middle Aviation Curriculum Guide, page 63, Fuel Consumption answers"


def vector(i, inputs, expected, source=SRC, version="2026"):
    expected = dict(expected)
    expected.setdefault("ok", True)
    tolerance = {
        key: {"abs": 1e-9, "rel": 1e-12}
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


def calculated(i, fuel, hours, usable=None, *, fuel_text=None, time_text=None, usable_text=None):
    inputs = {
        "fuel_used": fuel_text or f"{fuel} gal",
        "time": time_text or f"{hours} h",
    }
    expected = {"result.burn_rate.value": fuel / hours}
    if usable is not None:
        inputs["usable_fuel"] = usable_text or f"{usable} gal"
        expected["result.endurance.value"] = usable / (fuel / hours)
    return vector(i, inputs, expected)


def vectors():
    out = [
        vector(1, {"fuel_used": "28 gal", "time": "2.5 h", "usable_fuel": "40 gal"},
               {"result.burn_rate.value": 11.2, "result.endurance.value": 40 / 11.2}, SPEC),
        calculated(2, 9.5, 1.0),
        calculated(3, 45.2, 3.8, 12.0),
        calculated(4, 120, 1.75, 250),
        calculated(5, 3.1, 0.4, 48),
        vector(6, {"fuel_used": "0 gal", "time": "2 h"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/fuel_used"},
               "The tool's stated domain: no fuel used gives no rate"),
    ]

    # The handbook works this in the other direction: 5 gal/h for 4 h is 20 gal.
    out.append(vector(7, {"fuel_used": "20 gal", "time": "4 h"},
                      {"result.burn_rate.value": 5.0}, PHAK, "FAA-H-8083-25C (2023)"))
    # Three exact rows from the FAA curriculum's printed answer table.
    for fuel, hours, rate in [(21, 3.5, 6), (12, 2.4, 5), (48, 3.2, 15)]:
        out.append(vector(len(out) + 1, {"fuel_used": f"{fuel} gal", "time": f"{hours} h"},
                          {"result.burn_rate.value": float(rate)}, CURRICULUM, "1995"))

    out.extend([
        calculated(11, 10 / GAL_L, 0.75, 40 / GAL_L,
                   fuel_text="10 L", time_text="45 min", usable_text="40 L"),
        calculated(12, 5, 0.5, 30),
        calculated(13, 100, 10, 0),
        calculated(14, 0.1, 0.01, 1.5),
        calculated(15, 95, 8.25, 170),
        calculated(16, 360 / GAL_L, 6, 720 / GAL_L,
                   fuel_text="360 L", usable_text="720 L"),
        calculated(17, 7.25, 0.8),
        calculated(18, 64, 5.33, 80),
        vector(19, {"fuel_used": "8 gal", "time": "0 h"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/time"},
               "The tool's stated domain: time must be positive"),
        vector(20, {"fuel_used": "8 gal", "time": "1 h", "usable_fuel": "-1 gal"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/usable_fuel"},
               "The tool's stated domain: usable fuel cannot be negative"),
    ])
    return out


def main():
    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "aviation.loading.burn-rate.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors()))


if __name__ == "__main__":
    main()
