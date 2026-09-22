#!/usr/bin/env python3
"""Golden vectors for survey.gnss.localization, by NumPy least squares on the
full design matrix (independent of the tool's centroid-reduced closed form):
similarity [e -n 1 0; n e 0 1][a c tE tN] and affine, in meters."""
import json
import math
import sys
from pathlib import Path

import numpy as np

SRC, VER = "Least squares by NumPy lstsq (tools/vectors/gen_localize.py)", "NumPy 2.x"
SPEC = "add-practitioner-essentials localization scenarios"
FT = 0.3048


def fit(pairs, affine):
    """pairs: (local e, local n, grid e, grid n) in meters."""
    # Centered first, as large grid values would cost the solve its precision.
    m = [sum(p[k] for p in pairs) / len(pairs) for k in range(4)]
    pairs = [(p[0] - m[0], p[1] - m[1], p[2] - m[2], p[3] - m[3]) for p in pairs]
    A, y = [], []
    for le, ln, ge, gn in pairs:
        if affine:
            A += [[le, ln, 1, 0, 0, 0], [0, 0, 0, le, ln, 1]]
        else:
            A += [[le, -ln, 1, 0], [ln, le, 0, 1]]
        y += [ge, gn]
    x, *_ = np.linalg.lstsq(np.array(A, float), np.array(y, float), rcond=None)
    r = np.array(y) - np.array(A) @ x
    rms = math.sqrt(float(np.sum(r ** 2)) / len(pairs))
    if affine:
        a, b, _, c, d, _ = x
        return math.sqrt(abs(a * d - b * c)), math.degrees(math.atan2(c, a)), rms
    a, c = x[0], x[1]
    return math.hypot(a, c), math.degrees(math.atan2(c, a)), rms


def rows(pairs, unit):
    k = FT if unit == "ft" else 1.0
    out = [{"name": f"P{i + 1}", "local_n": f"{ln / k:.4f} {unit}", "local_e": f"{le / k:.4f} {unit}", "grid_n": f"{gn / k:.4f} {unit}", "grid_e": f"{ge / k:.4f} {unit}"}
           for i, (le, ln, ge, gn) in enumerate(pairs)]
    # What the tool reads back, after the rounding above.
    back = [(float(r["local_e"].split()[0]) * k, float(r["local_n"].split()[0]) * k, float(r["grid_e"].split()[0]) * k, float(r["grid_n"].split()[0]) * k) for r in out]
    return out, back


def make(s, th_deg, t, local, noise=None):
    th = math.radians(th_deg)
    out = []
    for i, (le, ln) in enumerate(local):
        dn, de = noise[i] if noise else (0.0, 0.0)
        out.append((le, ln, s * (math.cos(th) * le - math.sin(th) * ln) + t[0] + de, s * (math.sin(th) * le + math.cos(th) * ln) + t[1] + dn))
    return out


def vec(i, inp, exp, src=SRC):
    exp = dict(exp, ok=True) if "ok" not in exp else exp
    tol = {k: ({"abs": 1e-9} if k.endswith("scale") else {"abs": 1e-7} if "rotation" in k else {"abs": 1e-6}) for k, v in exp.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER, "tolerance": tol}


def main():
    out = []
    loc = [(1524.0, 1524.0), (1584.96, 1828.8), (1859.28, 1645.92), (1767.84, 1432.56)]
    cases = [
        (make(0.99997, 1.5, (956000.0, 463000.0), loc, [(0.0012, -0.0009), (-0.0006, 0.0015), (0.0003, -0.0012), (-0.0009, 0.0006)]), "ft", False, 0.99997, SRC),
        (make(0.99992, -0.75, (500000.0, 4000000.0), loc[:2]), "m", False, None, SPEC + " (two points: no redundancy)"),
        (make(1.00012, 12.0, (300000.0, 200000.0), loc, [(0.01, 0.0), (0.0, -0.01), (-0.005, 0.005), (0.004, 0.002)]), "m", True, 1.0001, SRC),
        (make(1.0, 0.0, (10.0, 20.0), loc[:3]), "m", True, None, SRC + " (three points: no redundancy)"),
        (make(0.3048, 2.0, (1000.0, 2000.0), loc), "m", False, 0.99997, SPEC + " (feet entered as meters)"),
        (make(0.9999, 45.0, (0.0, 0.0), loc + [(1600.0, 1600.0)]), "m", False, None, SRC),
    ]
    for i, (pairs, unit, affine, expected, src) in enumerate(cases, 1):
        r, back = rows(pairs, unit)
        scale, rot, rms = fit(back, affine)
        inp = {"pairs": r}
        if affine:
            inp["model"] = "affine"
        if expected is not None:
            inp["expected_scale"] = expected
        exp = {"result.scale": scale, "result.rotation.value": rot, "result.rms.value": rms}
        if len(pairs) == (3 if affine else 2):
            exp["meta.warnings.1.code"] = "NO_REDUNDANCY"
        if expected is not None and abs(scale / expected - 1) * 1e6 > 100:
            exp["meta.warnings.1.code"] = "SCALE_SUSPECT"
        out.append(vec(i, inp, exp, src))
    # Two points cannot fix an affine transform.
    r, _ = rows(cases[1][0], "m")
    out.append(vec(len(out) + 1, {"pairs": r, "model": "affine"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "survey.gnss.localization.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
