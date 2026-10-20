#!/usr/bin/env python3
"""Golden vectors for survey.cogo.helmert-3d-fit, worked in Python: target
points made from source points by the IOGP GN 7-2 linearized Helmert
transformation, V_T = T + (1 + dS)(I + Omega) V_S, with known parameters,
so the fit must recover them. Prints JSON lines; writes nothing."""
import json
import random
import sys

ARCSEC = 206264.80624709636


def transform(p, v, cf=False):
    tx, ty, tz, s_ppm, rx, ry, rz = p
    r = [rx / ARCSEC, ry / ARCSEC, rz / ARCSEC]
    if cf:
        r = [-x for x in r]
    x, y, z = v
    rv = [x - r[2] * y + r[1] * z, r[2] * x + y - r[0] * z, -r[1] * x + r[0] * y + z]
    k = 1 + s_ppm * 1e-6
    return [tx + k * rv[0], ty + k * rv[1], tz + k * rv[2]]


def points(seed, n, center=(4027894.0, 307045.6, 4919474.9), spread=2000.0):
    rnd = random.Random(seed)
    return [[c + rnd.uniform(-spread, spread) for c in center] for _ in range(n)]


def case(seed, n, p, cf=False, round_mm=None, center=None, spread=2000.0):
    src = points(seed, n, center or (4027894.0, 307045.6, 4919474.9), spread)
    rows = []
    for i, v in enumerate(src):
        t = transform(p, v, cf)
        if round_mm:
            t = [round(x, round_mm) for x in t]
        rows.append({"name": f"P{i + 1}", "x": f"{v[0]!r} m", "y": f"{v[1]!r} m", "z": f"{v[2]!r} m",
                     "x_target": f"{t[0]!r} m", "y_target": f"{t[1]!r} m", "z_target": f"{t[2]!r} m"})
    inp = {"points": rows}
    if cf:
        inp["convention"] = "coordinate-frame"
    return inp


SRC = "Synthetic points from known parameters by the IOGP GN 7-2 linearized Helmert formula, worked in Python (tools/vectors/gen_helmert_fit.py)"
PARAMS = [
    (-0.12, 0.08, 0.31, 0.85, 0.12, -0.08, 0.25),
    (446.448, -125.157, 542.06, -20.4894, 0.1502, 0.247, 0.8421),
    (0.0, 0.0, 4.5, 0.219, 0.0, 0.0, 0.554),
    (-87.0, -98.0, -121.0, 0.0, 0.0, 0.0, 0.0),
    (1.2, -0.7, 0.3, 3.1, -1.5, 2.25, -0.6),
]


def vectors():
    out = []
    i = 0
    for k, p in enumerate(PARAMS):
        for cf in (False, True):
            for n in (4, 7):
                i += 1
                inp = case(100 * k + n + (50 if cf else 0), n, p, cf)
                exp = {"ok": True, "result.tx.value": p[0], "result.ty.value": p[1], "result.tz.value": p[2],
                       "result.scale.value": p[3], "result.rx.value": p[4], "result.ry.value": p[5], "result.rz.value": p[6],
                       "result.degrees_of_freedom": float(3 * n - 7)}
                # At ECEF magnitudes (6e6 m) with points 2 km apart, the last bit
                # of a coordinate fixes the rotations to about 1e-7 arcsec, which
                # moves the translations by micrometers: 10 um is the honest bound.
                tol = {key: {"abs": 1e-5} for key in exp if key.startswith("result.t")}
                tol.update({key: {"abs": 1e-5} for key in ("result.rx.value", "result.ry.value", "result.rz.value", "result.scale.value")})
                tol["result.degrees_of_freedom"] = {"abs": 0}
                out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": "2023", "tolerance": tol})
    # A local 3D system (near the origin, 500 m across): recovered to 1e-9.
    for k, p in enumerate(PARAMS[:3]):
        i += 1
        inp = case(700 + k, 10, (p[0] % 7, p[1] % 7, p[2] % 7, p[3], p[4], p[5], p[6]), center=(0.0, 0.0, 0.0), spread=500.0)
        q = (p[0] % 7, p[1] % 7, p[2] % 7, p[3], p[4], p[5], p[6])
        exp = {"ok": True, "result.tx.value": q[0], "result.ty.value": q[1], "result.tz.value": q[2], "result.scale.value": q[3], "result.rx.value": q[4], "result.ry.value": q[5], "result.rz.value": q[6]}
        tol = {key: {"abs": 1e-9} for key in exp if key != "ok"}
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": "2023", "tolerance": tol})
    # Three points: the fit still runs, and warns.
    i += 1
    inp = case(901, 3, PARAMS[0])
    out.append({"id": f"v{i:03d}", "input": inp, "expect": {"ok": True, "meta.warnings.*.code": "NO_REDUNDANCY", "result.degrees_of_freedom": 2.0}, "source": SRC, "sourceVersion": "2023", "tolerance": {"result.degrees_of_freedom": {"abs": 0}}})
    # A wrong unit on the target side shows up as a scale far from one.
    i += 1
    inp = case(902, 5, (0, 0, 0, 1000.0, 0, 0, 0))
    out.append({"id": f"v{i:03d}", "input": inp, "expect": {"ok": True, "meta.warnings.*.code": "SCALE_SUSPECT"}, "source": SRC, "sourceVersion": "2023", "tolerance": {}})
    # Collinear points do not fix the rotations.
    i += 1
    rows = [{"name": f"L{j}", "x": f"{1000.0 * j} m", "y": f"{2000.0 * j} m", "z": f"{3000.0 * j} m", "x_target": f"{1000.0 * j + 1} m", "y_target": f"{2000.0 * j} m", "z_target": f"{3000.0 * j} m"} for j in range(4)]
    out.append({"id": f"v{i:03d}", "input": {"points": rows}, "expect": {"ok": False, "error.code": "DEGENERATE_GEOMETRY", "error.field": "/points"}, "source": SRC, "sourceVersion": "2023", "tolerance": {}})
    return out


if __name__ == "__main__":
    if sys.argv[1:] == ["example"]:
        print(json.dumps(case(7, 5, PARAMS[0], round_mm=4), separators=(",", ":")))
    else:
        for v in vectors():
            print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
