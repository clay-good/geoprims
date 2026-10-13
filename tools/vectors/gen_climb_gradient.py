#!/usr/bin/env python3
"""Golden vectors for aviation.performance.climb-gradient."""
import json
import math
import sys
from pathlib import Path

OLD_SRC = "Independent Python implementation of the turn, gradient, glide, and pivotal-altitude relations in FAA-H-8083-3C and FAA-H-8083-16B (tools/vectors/gen_aviation.py)"
OLD_VER = "FAA-H-8083-16B (2017)"
SRC = "Climb-gradient relations evaluated in Python (tools/vectors/gen_climb_gradient.py)"
FAA = "FAA Instrument Procedures Handbook (FAA-H-8083-16B), chapter 1, figure 1-17"
FT_PER_NM = 1852.0 / 0.3048


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


def fpm_from_ft_nm(gradient, groundspeed):
    return gradient * groundspeed / 60.0


def fpm_from_angle(angle, groundspeed):
    return math.tan(math.radians(angle)) * FT_PER_NM * groundspeed / 60.0


def vectors():
    # Preserve the five originally published vectors byte for byte.
    out = [
        vector(1, {"gradient": "200 ft/NM", "groundspeed": "120 kt"}, {"result.vertical_speed.value": 400.0, "result.angle.value": 1.885253852180525}, OLD_SRC, OLD_VER),
        vector(2, {"gradient": "300 ft/NM", "groundspeed": "90 kt"}, {"result.vertical_speed.value": 450.0, "result.angle.value": 2.8266064014033825}, OLD_SRC, OLD_VER),
        vector(3, {"gradient": "425 ft/NM", "groundspeed": "160 kt"}, {"result.vertical_speed.value": 1133.3333333333333, "result.angle.value": 4.001094289869373}, OLD_SRC, OLD_VER),
        vector(4, {"gradient": "152 ft/NM", "groundspeed": "200 kt"}, {"result.vertical_speed.value": 506.6666666666667, "result.angle.value": 1.4330113542979013}, OLD_SRC, OLD_VER),
        vector(5, {"gradient": "500 ft/NM", "groundspeed": "75 kt"}, {"result.vertical_speed.value": 625.0, "result.angle.value": 4.704237067623307}, OLD_SRC, OLD_VER),
    ]
    out.append(vector(6, {"angle": "2.8 deg", "groundspeed": "180 kt"},
                      {"result.vertical_speed.value": 892.0}, FAA, OLD_VER, abs_tol=0.5))
    for gradient, groundspeed in [(200, 60), (200, 180), (297, 180), (351, 90), (425, 240)]:
        out.append(vector(len(out) + 1,
                          {"gradient": f"{gradient} ft/NM", "groundspeed": f"{groundspeed} kt"},
                          {"result.vertical_speed.value": fpm_from_ft_nm(gradient, groundspeed)}))
    for angle, groundspeed in [(3.0, 120), (5.0, 90), (7.5, 240)]:
        out.append(vector(len(out) + 1,
                          {"angle": f"{angle} deg", "groundspeed": f"{groundspeed} kt"},
                          {"result.vertical_speed.value": fpm_from_angle(angle, groundspeed)}))
    for vertical_speed, groundspeed in [(500, 100), (800, 160)]:
        out.append(vector(len(out) + 1,
                          {"vertical_speed": f"{vertical_speed} fpm", "groundspeed": f"{groundspeed} kt"},
                          {"result.gradient.value": vertical_speed * 60.0 / groundspeed}))
    out.extend([
        vector(17, {"groundspeed": "120 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/gradient"}, "The tool's stated domain"),
        vector(18, {"gradient": "200 ft/NM", "angle": "2 deg", "groundspeed": "120 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/gradient"}, "The tool's stated domain"),
        vector(19, {"gradient": "0 ft/NM", "groundspeed": "120 kt"},
               {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/gradient"}, "The tool's stated domain"),
        vector(20, {"angle": "90 deg", "groundspeed": "120 kt"},
               {"ok": False, "error.code": "OUT_OF_DOMAIN", "error.field": "/angle"}, "The tool's stated domain"),
    ])
    assert len(out) == 20
    return out


def main():
    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "aviation.performance.climb-gradient.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vectors()))


if __name__ == "__main__":
    main()
