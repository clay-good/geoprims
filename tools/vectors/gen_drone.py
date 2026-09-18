#!/usr/bin/env python3
"""Golden vectors for drone.photogrammetry.* by direct evaluation of the
published formulas (Wolf, Dewitt & Wilkinson 2014, ch. 6; ASPRS Edition 2),
plus the add-drone-suite spec scenarios."""
import json
import math
import sys
from pathlib import Path

SRC = "Photogrammetric formulas (Wolf, Dewitt & Wilkinson 2014, ch. 6) evaluated in Python (tools/vectors/gen_drone.py)"
SPEC = "add-drone-suite scenarios"
CAMS = [  # sensor w, h (mm), focal (mm), image w, h (px)
    (13.2, 8.8, 8.8, 5472, 3648),    # 1-inch 20 MP
    (17.3, 13.0, 12.29, 5280, 3956),  # Four Thirds 20 MP
    (6.17, 4.55, 4.5, 4000, 3000),   # 1/2.3-inch 12 MP
    (35.9, 24.0, 35.0, 8192, 5460),  # full frame 45 MP
    (23.5, 15.6, 16.0, 6000, 4000),  # APS-C 24 MP
]


def vec(i, inp, exp, src=SRC, ver="4th edition (2014)"):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": 1e-12, "abs": 1e-12} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def cam_inp(c):
    return {"sensor_width": f"{c[0]} mm", "sensor_height": f"{c[1]} mm", "focal_length": f"{c[2]} mm", "image_width": c[3], "image_height": c[4]}


def gsd():
    out = []
    for i, (c, h) in enumerate(zip(CAMS, [100, 80, 50, 300, 120]), 1):
        g = c[0] * h / (c[2] * c[3]) * 100  # cm
        out.append(vec(i, dict(cam_inp(c), height=f"{h} m"),
                       {"result.gsd.value": g, "result.footprint_across.value": c[0] * h / c[2], "result.gsd_along.value": c[1] * h / (c[2] * c[4]) * 100}))
    out.append(vec(6, dict(cam_inp((13.2, 8.8, 24.0, 5472, 3648)), height="100 m"), {"meta.warnings.0.code": "EQUIVALENT_FOCAL_LENGTH"}, SPEC, "2026"))
    return out


def alt():
    out = []
    for i, (c, g) in enumerate(zip(CAMS, [2.0, 1.5, 3.0, 0.8, 2.5]), 1):
        inp = cam_inp(c)
        inp["target_gsd"] = f"{g} cm"
        out.append(vec(i, inp, {"result.height.value": g / 100 * c[2] * c[3] / c[0]}))
    return out


def trigger():
    out = []
    cases = [(CAMS[0], 100, 10, 75, 65), (CAMS[1], 80, 8, 80, 70), (CAMS[2], 60, 5, 85, 70), (CAMS[3], 200, 15, 70, 60), (CAMS[4], 120, 12, 75, 60)]
    for i, (c, h, v, fo, so) in enumerate(cases, 1):
        across, along = c[0] * h / c[2], c[1] * h / c[2]
        d = along * (1 - fo / 100)
        inp = dict(cam_inp(c), height=f"{h} m", groundspeed=f"{v} m/s", front_overlap=fo, side_overlap=so)
        del inp["image_height"]
        out.append(vec(i, inp, {"result.trigger_distance.value": d, "result.trigger_interval.value": d / v, "result.line_spacing.value": across * (1 - so / 100)}))
    return out


def blur():
    cases = [(10, 0.001, 2.741), (15, 1 / 2000, 1.0), (5, 1 / 500, 3.0), (20, 1 / 4000, 0.8), (8, 1 / 800, 2.0)]
    out = []
    for i, (v, t, g) in enumerate(cases, 1):
        out.append(vec(i, {"groundspeed": f"{v} m/s", "exposure": f"{t} s", "gsd": f"{g} cm"},
                       {"result.blur": v * t / (g / 100), "result.max_exposure.value": 0.5 * (g / 100) / v}))
    return out


def asprs():
    cases = [((None, None, 1.0), 2.0, 30), ((1.0, 1.0, None), 1.0, 40), ((2.0, 1.5, 3.0), 1.2, 35), ((0.5, 0.5, 0.5), 0.3, 30), ((None, None, 5.0), 0.0, 100)]
    out = []
    for i, ((x, y, z), cp, n) in enumerate(cases, 1):
        inp = {"checkpoint_rmse": f"{cp} cm", "checkpoints": n}
        exp = {}
        if x is not None:
            inp["rmse_x"], inp["rmse_y"] = f"{x} cm", f"{y} cm"
            exp["result.horizontal.value"] = math.hypot(math.sqrt(x * x + cp * cp), math.sqrt(y * y + cp * cp))
        if z is not None:
            inp["rmse_z"] = f"{z} cm"
            exp["result.vertical.value"] = math.sqrt(z * z + cp * cp)
        out.append(vec(i, inp, exp, "ASPRS Positional Accuracy Standards, Edition 2 (quadrature of fit and checkpoint RMSE)", "Edition 2, Version 2.0"))
    out.append(vec(6, {"rmse_z": "1 cm", "checkpoint_rmse": "1 cm", "checkpoints": 20}, {"meta.warnings.1.code": "INSUFFICIENT_CHECKPOINTS"}, SPEC, "2026"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {"drone.photogrammetry.gsd": gsd(), "drone.photogrammetry.altitude-for-gsd": alt(), "drone.photogrammetry.trigger": trigger(),
             "drone.photogrammetry.motion-blur": blur(), "drone.photogrammetry.asprs-accuracy": asprs()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
