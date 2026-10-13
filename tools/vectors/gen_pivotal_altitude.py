#!/usr/bin/env python3
"""Golden vectors for aviation.performance.pivotal-altitude."""
import json
import sys
from pathlib import Path

OLD_SRC = "Independent Python implementation of the turn, gradient, glide, and pivotal-altitude relations in FAA-H-8083-3C and FAA-H-8083-16B (tools/vectors/gen_aviation.py)"
OLD_VER = "FAA-H-8083-3C (2021)"
SRC = "Pivotal-altitude relations evaluated in Python (tools/vectors/gen_pivotal_altitude.py)"
FAA = "FAA Airplane Flying Handbook (FAA-H-8083-3C), chapter 7, figure 7-12"
KT = 1852.0 / 3600.0
FT = 0.3048
G0 = 9.80665


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


def exact_height(knots):
    return (knots * KT) ** 2 / G0 / FT


def vectors():
    # Preserve the five originally published vectors byte for byte.
    out = [
        vector(1, {"groundspeed": "100 kt"}, {"result.pivotal_altitude.value": 885.4036845041701}, OLD_SRC, OLD_VER),
        vector(2, {"groundspeed": "80 kt"}, {"result.pivotal_altitude.value": 566.6583580826689}, OLD_SRC, OLD_VER),
        vector(3, {"groundspeed": "120 kt"}, {"result.pivotal_altitude.value": 1274.9813056860048}, OLD_SRC, OLD_VER),
        vector(4, {"groundspeed": "95 kt"}, {"result.pivotal_altitude.value": 799.0768252650134}, OLD_SRC, OLD_VER),
        vector(5, {"groundspeed": "110 kt"}, {"result.pivotal_altitude.value": 1071.3384582500457}, OLD_SRC, OLD_VER),
    ]
    out.append(vector(6, {"groundspeed": "100 kt"},
                      {"result.pivotal_altitude.value": 885.0}, FAA, OLD_VER, abs_tol=0.5))
    for knots in [1, 20, 40, 60, 75, 87, 104, 113, 130, 150, 200, 250]:
        out.append(vector(len(out) + 1, {"groundspeed": f"{knots} kt"}, {
            "result.pivotal_altitude.value": exact_height(knots),
            "result.rule_of_thumb.value": knots * knots / 11.3,
        }))
    out.extend([
        vector(19, {"groundspeed": "0 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/groundspeed"}, "The tool's stated domain"),
        vector(20, {"groundspeed": "-1 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/groundspeed"}, "The tool's stated domain"),
    ])
    assert len(out) == 20
    return out


def main():
    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "aviation.performance.pivotal-altitude.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors()))


if __name__ == "__main__":
    main()
