#!/usr/bin/env python3
"""Golden vectors for aviation.loading.fuel-weight."""
import json
import sys
from pathlib import Path

WB_SRC = "Independent Python weight-and-balance arithmetic per FAA-H-8083-1B chapter 2 (tools/vectors/gen_aviation.py)"
WB_VER = "FAA-H-8083-1B (2016)"
SRC = "Fuel volume and weight arithmetic evaluated in Python (tools/vectors/gen_fuel_weight.py)"
PHAK = "FAA Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), chapter 10, figures 10-5 and 10-6"


def vector(i, inputs, expected, source=SRC, version="2026", rel=1e-12, abs_tol=1e-6):
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


def vectors():
    # Preserve the five originally published vectors byte for byte.
    out = [
        vector(1, {"volume": "40 gal", "fuel": "100ll"}, {"result.weight.value": 240.0}, WB_SRC, WB_VER),
        vector(2, {"volume": "100 gal", "fuel": "jet-a"}, {"result.weight.value": 670.0}, WB_SRC, WB_VER),
        vector(3, {"volume": "56 gal", "fuel": "100ll"}, {"result.weight.value": 336.0}, WB_SRC, WB_VER),
        vector(4, {"volume": "30 gal", "density": "5.9 lb/gal"}, {"result.weight.value": 177.0}, WB_SRC, WB_VER),
        vector(5, {"weight": "300 lb", "fuel": "100ll"}, {"result.volume.value": 50.0}, WB_SRC, WB_VER),
    ]
    for gallons, pounds in [(40, 240), (75, 450)]:
        out.append(vector(len(out) + 1, {"volume": f"{gallons} gal", "fuel": "100ll"},
                          {"result.weight.value": pounds}, PHAK, "FAA-H-8083-25C (2023)", abs_tol=0))
    for gallons, fuel, density in [
        (0, "100ll", 6.0),
        (1, "100ll", 6.0),
        (5.5, "100ll", 6.0),
        (42, "100ll", 6.0),
        (1, "jet-a", 6.7),
        (50, "jet-a", 6.7),
        (200, "jet-a", 6.7),
    ]:
        out.append(vector(len(out) + 1, {"volume": f"{gallons} gal", "fuel": fuel},
                          {"result.weight.value": gallons * density}))
    for pounds, fuel, density in [(6, "100ll", 6.0), (67, "jet-a", 6.7)]:
        out.append(vector(len(out) + 1, {"weight": f"{pounds} lb", "fuel": fuel},
                          {"result.volume.value": pounds / density}))
    out.extend([
        vector(17, {"volume": "20 gal", "fuel": "100ll", "density": "5.8 lb/gal"},
               {"result.weight.value": 116.0}),
        vector(18, {"volume": "10 gal", "weight": "60 lb", "fuel": "100ll"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/volume"},
               "The tool's stated domain"),
        vector(19, {"volume": "-1 gal", "fuel": "100ll"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/volume"},
               "The tool's stated domain"),
        vector(20, {"volume": "10 gal", "density": "0 lb/gal"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/density"},
               "The tool's stated domain"),
    ])
    assert len(out) == 20
    return out


def main():
    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "aviation.loading.fuel-weight.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors()))


if __name__ == "__main__":
    main()
