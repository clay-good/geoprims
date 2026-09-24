#!/usr/bin/env python3
"""Golden vectors for drone.photogrammetry.asprs-accuracy (version 1.1.0).

Appends to core/vectors/drone.photogrammetry.asprs-accuracy.jsonl, leaving
v001 to v006 as they are apart from marking v006 superseded:

- Table 7.4 of the ASPRS Positional Accuracy Standards, Edition 2 (Version
  1.0, February 2023), all 19 rows: vertical product accuracy from the fit to
  checkpoints and a 2.0 cm checkpoint survey, to the table's printed 0.01 cm.
- Checkpoint-error lists, with the statistics recomputed here from the
  standard's definitions (section 7.2 mean error and blunders, 7.4 NVA and
  VVA, 7.11 product accuracy, 7.12 checkpoint accuracy, 7.13 the minimum of
  30): RMSE = sqrt(sum(e^2) / n), horizontal per-axis target = class / sqrt(2).

Run from the repository root."""
import json
import math
import random
from pathlib import Path

PATH = Path("core/vectors/drone.photogrammetry.asprs-accuracy.jsonl")
TABLE = "ASPRS Positional Accuracy Standards, Edition 2, Table 7.4 (Computing Vertical Product Accuracy)"
TABLE_VER = "Edition 2, Version 1.0 (February 2023)"
SRC = "Recomputed from the ASPRS Edition 2 definitions (sections 7.2, 7.4, 7.11 to 7.13) by tools/vectors/gen_asprs.py"

TABLE_7_4 = [(1.00, 2.24), (1.50, 2.50), (2.00, 2.83), (2.50, 3.20), (3.00, 3.61), (3.50, 4.03), (4.00, 4.47),
             (4.50, 4.92), (5.00, 5.39), (5.50, 5.85), (6.00, 6.32), (6.50, 6.80), (7.00, 7.28), (7.50, 7.76),
             (8.00, 8.25), (8.50, 8.73), (9.00, 9.22), (9.50, 9.71), (10.00, 10.20)]


def rms(v):
    return math.sqrt(sum(e * e for e in v) / len(v))


def vec(i, inp, exp, src=SRC, ver=TABLE_VER, abs_tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 1e-9 if abs_tol < 1e-6 else 0, "abs": abs_tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def row(dx=None, dy=None, dz=None, cover=None):
    r = {}
    if dx is not None:
        r["dx"], r["dy"] = f"{dx} cm", f"{dy} cm"
    if dz is not None:
        r["dz"] = f"{dz} cm"
    if cover:
        r["cover"] = cover
    return r


def main():
    lines = PATH.read_text().splitlines()
    old = [json.loads(l) for l in lines]
    assert [v["id"] for v in old[:6]] == [f"v{i:03d}" for i in range(1, 7)]
    out = old[:6]
    n = len(out)

    def add(inp, exp, **kw):
        out.append(vec(len(out) + 1, inp, exp, **kw))

    # v006 pinned the warning's index, which moved when EXPERIMENTAL_TOOL was
    # dropped on promotion; the replacement matches the code at any index.
    if "supersededBy" not in out[5]:
        out[5]["supersededBy"] = "v007"
        out[5]["reason"] = "The tool was promoted to stable (1.1.0) and no longer carries EXPERIMENTAL_TOOL, so INSUFFICIENT_CHECKPOINTS moved from the second warning to the first. The result is unchanged; v007 matches the code at any index."
    add(out[5]["input"], {"meta.warnings.*.code": "INSUFFICIENT_CHECKPOINTS"}, src="add-drone-suite scenarios", ver="2026")

    for fit, product in TABLE_7_4:
        add({"rmse_z": f"{fit:.2f} cm", "checkpoint_rmse": "2.0 cm", "checkpoints": 30},
            {"result.vertical.value": product}, src=TABLE, abs_tol=0.005)

    rng = random.Random(107)
    # 30 open checkpoints, with a 5 cm horizontal and vertical class and 1 cm
    # checkpoints: meets both classes, no blunders, small bias.
    pts = [(round(rng.gauss(0, 1.5), 1), round(rng.gauss(0, 1.5), 1), round(rng.gauss(0, 2.0), 1)) for _ in range(30)]
    xs, ys, zs = zip(*pts)
    h = math.hypot(math.sqrt(rms(xs) ** 2 + 1.0), math.sqrt(rms(ys) ** 2 + 1.0))
    v = math.sqrt(rms(zs) ** 2 + 1.0)
    add({"errors": [row(*p) for p in pts], "checkpoint_rmse": "1 cm", "target_horizontal": "5 cm", "target_vertical": "5 cm"},
        {"result.horizontal.value": h, "result.vertical.value": v, "result.mean_z.value": sum(zs) / 30,
         "result.horizontal_result": "meets the 5 cm class" if h <= 5 else "does not meet the 5 cm class",
         "result.vertical_result": "meets the 5 cm class" if v <= 5 else "does not meet the 5 cm class",
         "result.checkpoint_status": "The checkpoint count meets the Edition 2 minimum of 30."})
    # The same with 12 vegetated height checkpoints added: VVA reported, NVA
    # unchanged, and only 12 in the VVA test.
    veg = [round(rng.gauss(3, 6), 1) for _ in range(12)]
    add({"errors": [row(*p) for p in pts] + [row(dz=z, cover="vegetated") for z in veg], "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"},
        {"result.vertical.value": v, "result.vva.value": math.sqrt(rms(veg) ** 2 + 1.0), "meta.warnings.*.code": "INSUFFICIENT_CHECKPOINTS",
         "result.checkpoint_status": "12 checkpoints for the VVA test is below the Edition 2 minimum of 30."})
    # A blunder: one height error of 16 cm against a 5 cm class (limit 15 cm).
    blunder = list(pts)
    blunder[6] = (blunder[6][0], blunder[6][1], 16.0)
    zb = [p[2] for p in blunder]
    add({"errors": [row(*p) for p in blunder], "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"},
        {"result.vertical.value": math.sqrt(rms(zb) ** 2 + 1.0), "result.blunders.0.checkpoint": 7,
         "result.blunders.0.component": "height", "result.blunders.0.error.value": 16.0, "result.blunders.0.limit.value": 15.0,
         "meta.warnings.*.code": "CHECKPOINT_BLUNDER"}, src="add-drone-suite scenario (blunder detection), recomputed by tools/vectors/gen_asprs.py")
    # A horizontal blunder: easting 11 cm against a 5 cm RMSE_H class (per
    # axis 3.54 cm, limit 10.61 cm).
    hb = list(pts)
    hb[2] = (11.0, hb[2][1], hb[2][2])
    add({"errors": [row(*p) for p in hb], "checkpoint_rmse": "1 cm", "target_horizontal": "5 cm"},
        {"result.blunders.0.checkpoint": 3, "result.blunders.0.component": "easting",
         "result.blunders.0.limit.value": 3 * 5 / math.sqrt(2), "meta.warnings.*.code": "CHECKPOINT_BLUNDER"})
    # A bias: every height 1.5 cm high, over 25% of a 5 cm class.
    biased = [(p[0], p[1], round(p[2] + 1.5, 1)) for p in pts]
    zb = [p[2] for p in biased]
    add({"errors": [row(*p) for p in biased], "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"},
        {"result.mean_z.value": sum(zb) / 30, "meta.warnings.*.code": "MEAN_ERROR_HIGH"})
    # Checkpoints not twice as accurate: 2 cm checkpoints for a 3 cm class.
    add({"rmse_z": "1 cm", "checkpoint_rmse": "2 cm", "checkpoints": 40, "target_vertical": "3 cm"},
        {"result.vertical.value": math.sqrt(5), "result.vertical_result": "meets the 3 cm class",
         "meta.warnings.*.code": "CHECKPOINTS_TOO_COARSE"})
    # A class not met.
    add({"rmse_x": "4 cm", "rmse_y": "3 cm", "checkpoint_rmse": "1 cm", "checkpoints": 30, "target_horizontal": "5 cm"},
        {"result.horizontal.value": math.sqrt(16 + 1 + 9 + 1), "result.horizontal_result": "does not meet the 5 cm class"})
    # Errors: both a list and RMSEs; a horizontal row missing its northing.
    add({"errors": [row(dz=1.0)], "rmse_z": "1 cm", "checkpoint_rmse": "1 cm"}, {"ok": False, "error.code": "INVALID_INPUT"}, src="add-drone-suite scenarios", ver="2026")
    add({"errors": [{"dx": "1 cm"}], "checkpoint_rmse": "1 cm"}, {"ok": False, "error.code": "INVALID_INPUT"}, src="add-drone-suite scenarios", ver="2026")
    PATH.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    print(len(out) - n, "vectors appended;", len(out), "in all")


if __name__ == "__main__":
    main()
