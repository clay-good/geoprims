#!/usr/bin/env python3
"""Golden vectors for navigation.los.* (horizon, visibility, dip, Fresnel),
evaluated in Python from the spherical-Earth formulas with effective radius
R/(1 - k). Writes core/vectors/navigation.los.*.jsonl."""
import json
import math
import random
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/vectors"
R = 6_371_000.0
SRC = "Spherical Earth with effective radius R/(1 - k), evaluated in Python (tools/vectors/gen_los.py)"


def vec(i, inp, exp, rel=1e-12):
    e = dict(exp)
    e["ok"] = True
    tol = {k: {"rel": rel, "abs": 1e-9} for k, v in exp.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": "2026", "tolerance": tol}


def arc(re, h):
    return re * math.acos(re / (re + h))


def write(tool, rows):
    (OUT / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in rows))
    print(len(rows), "->", tool)


def main():
    rnd = random.Random(31)
    rows = []
    for i, h in enumerate([100.0, 2.0, 10.0] + [10 ** rnd.uniform(-1, 4) for _ in range(19)], 1):
        k = 0.13
        re = R / (1 - k)
        rows.append(vec(i, {"height": f"{h!r} m"}, {"result.optical.value": arc(re, h) / 1000, "result.geometric.value": arc(R, h) / 1000,
                                                    "result.radio.value": arc(R / 0.75, h) / 1000, "result.optical_slant.value": math.sqrt(2 * re * h + h * h) / 1000}, 1e-11))
    write("navigation.los.horizon", rows)

    rows = []
    cases = [(2.0, 50.0, 30_000.0)] + [(10 ** rnd.uniform(0, 3), 10 ** rnd.uniform(0, 3), rnd.uniform(1e3, 2e5)) for _ in range(21)]
    for i, (h1, h2, d) in enumerate(cases, 1):
        re = R / 0.87
        d1, d2 = arc(re, h1), arc(re, h2)
        hidden = re / math.cos((d - d1) / re) - re if d > d1 else 0.0
        rows.append(vec(i, {"observer_height": f"{h1!r} m", "target_height": f"{h2!r} m", "distance": f"{d / 1000!r} km"},
                        {"result.max_range.value": (d1 + d2) / 1000, "result.observer_horizon.value": d1 / 1000,
                         "result.hidden_height.value": hidden, "result.visible": "yes" if d <= d1 + d2 else "no"}, 1e-9))
    write("navigation.los.visibility", rows)

    rows = []
    for i, h in enumerate([10.0] + [10 ** rnd.uniform(-1, 3) for _ in range(21)], 1):
        re = R / 0.87
        dip = math.degrees(math.acos(re / (re + h))) * 60
        rows.append(vec(i, {"height": f"{h!r} m"}, {"result.dip.value": dip, "result.rule.value": 1.76 * math.sqrt(h)}, 1e-10))
    write("navigation.los.dip", rows)

    rows = []
    cases = [(5.8e9, 10_000.0, None)] + [(10 ** rnd.uniform(8, 10.5), rnd.uniform(500, 1e5), rnd.random()) for _ in range(21)]
    for i, (f, d, frac) in enumerate(cases, 1):
        d1 = d / 2 if frac is None else d * frac
        d2 = d - d1
        lam = 299_792_458.0 / f
        f1 = math.sqrt(lam * d1 * d2 / d)
        bulge = d1 * d2 / (2 * R / 0.75)
        inp = {"frequency": f"{f / 1e9!r} GHz", "distance": f"{d / 1000!r} km"}
        if frac is not None:
            inp["position"] = f"{d1 / 1000!r} km"
        rows.append(vec(i, inp, {"result.fresnel_radius.value": f1, "result.earth_bulge.value": bulge, "result.required_clearance.value": 0.6 * f1 + bulge}, 1e-10))
    write("navigation.los.fresnel", rows)


if __name__ == "__main__":
    main()
