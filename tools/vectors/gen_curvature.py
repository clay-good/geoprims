#!/usr/bin/env python3
"""Golden vectors for raster.terrain.curvature from analytic derivatives.

Each case is a surface z = A x²y² + B x²y + C xy² + D x² + E y² + F xy + G x + H y + I
(Zevenbergen and Thorne's partial quartic, x east and y north), sampled at the
nine cells of a 3 x 3 window. Its derivatives at the center are known from the
coefficients alone: z_x = G, z_y = H, z_xx = 2D, z_yy = 2E, z_xy = F, and the
expected curvatures are worked from those, never from the sampled grid:

  total   = -(z_xx + z_yy)
  profile = -(z_xx p² + 2 z_xy p q + z_yy q²) / (p² + q²)
  plan    = -(z_xx q² - 2 z_xy p q + z_yy p²) / (p² + q²)

with p = z_x, q = z_y, each times 100 (per 100 m), positive for convex. The
A, B, and C terms change the grid but not the center derivatives, which checks
that the tool reads them out of the window correctly.

Usage: python3 tools/vectors/gen_curvature.py
"""
import json
import math
import random
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/vectors/raster.terrain.curvature.jsonl"
WGS84_A = 6378137.0
WGS84_F = 1 / 298.257223563


def degree_lengths(lat):
    e2 = WGS84_F * (2 - WGS84_F)
    s = math.sin(math.radians(lat))
    w = math.sqrt(1 - e2 * s * s)
    m = WGS84_A * (1 - e2) / w**3
    n = WGS84_A / w
    return math.radians(m), math.radians(n) * math.cos(math.radians(lat))


def case(c, dx, dy, cell, note, decimals=None):
    A, B, Cc, D, E, F, G, H, I = c

    def z(x, y):
        v = A * x * x * y * y + B * x * x * y + Cc * x * y * y + D * x * x + E * y * y + F * x * y + G * x + H * y + I
        return round(v, decimals) if decimals is not None else v

    rows = []
    for y in (dy, 0.0, -dy):
        rows.append({"row": ", ".join(repr(z(x, y)) for x in (-dx, 0.0, dx))})
    zxx, zyy, zxy, p, q = 2 * D, 2 * E, F, G, H
    exp = {"ok": True, "result.curvature": float(-(zxx + zyy) * 100)}
    if p or q:
        n = p * p + q * q
        exp["result.profile"] = -(zxx * p * p + 2 * zxy * p * q + zyy * q * q) / n * 100
        exp["result.plan"] = -(zxx * q * q - 2 * zxy * p * q + zyy * p * p) / n * 100
    else:
        exp["meta.warnings.*.code"] = "FLAT_CELL"
    inp = {"elevations": rows, **cell}
    tol = {k: {"rel": 1e-9, "abs": 1e-9} for k, v in exp.items() if isinstance(v, float)}
    return {
        "input": inp,
        "expect": exp,
        "source": f"Analytic derivatives of a partial quartic sampled on the window (tools/vectors/gen_curvature.py): {note}",
        "sourceVersion": "2026",
        "tolerance": tol,
    }


CASES = [
    # The tool's primary example: 25% slope to the south, steepening downhill,
    # contours curling into a hollow.
    case((0, 0, 0, 0.003, -0.002, 0.0005, 0.02, 0.25, 103.0), 10, 10, {"cell_size": "10 m"},
         "a slope steepening into a hollow, 10 m cells", decimals=6),
    case((0, 0, 0, -0.01, -0.01, 0, 0, 0, 500.0), 30, 30, {"cell_size": "30 m"},
         "a rounded summit, flat at the center"),
    case((0, 0, 0, 0.004, 0.004, 0, 0, 0, 50.0), 5, 5, {"cell_size": "5 m"}, "the bottom of a bowl"),
    case((0, 0, 0, 0, 0, 0, 0.1, -0.05, 200.0), 30, 30, {"cell_size": "30 m"}, "a plane: no curvature"),
    case((0, 0, 0, 0.002, -0.002, 0, 0, 0, 80.0), 10, 10, {"cell_size": "10 m"}, "a saddle at its center"),
    case((0, 0, 0, 0, -0.003, 0, 0, -0.3, 150.0), 10, 10, {"cell_size": "10 m"},
         "a slope to the north rounding over a shoulder"),
    case((0, 0, 0, -0.003, 0, 0, 0, -0.3, 150.0), 10, 10, {"cell_size": "10 m"},
         "a spur running north: contours bow outward"),
    case((0, 0, 0, 0.003, 0, 0, 0, -0.3, 150.0), 10, 10, {"cell_size": "10 m"},
         "a hollow running north: contours curl inward"),
    case((0.00001, -0.0002, 0.0003, 0.001, -0.004, 0.002, 0.15, 0.05, 321.0), 10, 10, {"cell_size": "10 m"},
         "the quartic terms change the grid but not the center derivatives"),
]

rng = random.Random(20261009)
for i in range(11):
    dx = rng.choice([1.0, 2.0, 5.0, 10.0, 30.0])
    coeffs = (rng.uniform(-1e-5, 1e-5), rng.uniform(-1e-4, 1e-4), rng.uniform(-1e-4, 1e-4),
              rng.uniform(-0.01, 0.01), rng.uniform(-0.01, 0.01), rng.uniform(-0.01, 0.01),
              rng.uniform(-0.6, 0.6), rng.uniform(-0.6, 0.6), rng.uniform(0, 2000))
    CASES.append(case(coeffs, dx, dx, {"cell_size": f"{dx} m"}, f"random surface {i + 1}"))

# Geographic grids: one arc second at three latitudes, so the east-west cell
# narrows with the cosine of the latitude.
for lat in (0.0, 45.0, 60.0):
    deg = 1 / 3600
    per_lat, per_lon = degree_lengths(lat)
    dx, dy = deg * per_lon, deg * per_lat
    coeffs = (0, 0, 0, 0.004, -0.006, 0.003, 0.12, -0.2, 900.0)
    CASES.append(case(coeffs, dx, dy, {"cell_degrees": deg, "lat": f"{lat} deg"},
                      f"one arc second at {lat:g}° latitude, cells {dx:.3f} m by {dy:.3f} m"))

with OUT.open("w") as f:
    for i, row in enumerate(CASES, 1):
        f.write(json.dumps({"id": f"v{i:03d}", **row}) + "\n")
print(f"wrote {len(CASES)} vectors to {OUT}")
