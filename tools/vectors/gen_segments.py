#!/usr/bin/env python3
"""Golden vectors for navigation.geodesic.intersection and vertex, WGS 84.

Intersections come from GeographicLib's IntersectTool -i (Karney 2024), which
returns the crossing nearest the segments' midpoints and a flag k that is 0
when it lies within both. Vertices come from GeodSolve -a (the direct problem
in arc length) to σ = 90°, and each is checked here against the closed form
for the vertex latitude, cos β_v = |sin α₀|, and a due east or west azimuth.
Numbers go in fixed-point because GeographicLib reads an exponent's "e" as a
hemisphere."""
import json
import math
import random
import subprocess
import sys
from pathlib import Path

F = 1 / 298.257223563
SRC_I, SRC_V = "GeographicLib IntersectTool -i (Karney 2024, geodesic intersections)", "GeographicLib GeodSolve -a (Karney 2013), checked against Clairaut's relation"
VER = "GeographicLib 2.x"


def run(cmd, lines):
    out = subprocess.run(cmd, input="".join(l + "\n" for l in lines), capture_output=True, text=True, check=True).stdout
    return [[float(v) for v in l.split()] for l in out.splitlines() if l.strip()]


def fx(v):
    return f"{v:.12f}"


def ll_of(p0, x, lines_azi):
    """The crossing's latitude and longitude, x meters along from p0 at azi."""
    return run(["GeodSolve", "-p", "12"], [f"{fx(p0[0])} {fx(p0[1])} {fx(lines_azi)} {fx(x)}"])[0][:2]


def seg_vectors():
    rnd = random.Random(20260922)
    pairs = []
    for _ in range(60):
        lat, lon = rnd.uniform(-60, 60), rnd.uniform(-180, 180)
        pt = lambda: (max(-85, min(85, lat + rnd.uniform(-25, 25))), ((lon + rnd.uniform(-30, 30) + 180) % 360) - 180)
        pairs.append((pt(), pt(), pt(), pt()))
    # The spec's non-intersecting scenario: the full geodesics cross outside both segments.
    pairs.insert(0, ((0.0, 0.0), (0.0, 10.0), (10.0, 20.0), (5.0, 20.0)))
    res = run(["IntersectTool", "-i", "-p", "9"], [" ".join(fx(v) for p in q for v in p) for q in pairs])
    azis = run(["GeodSolve", "-i", "-p", "12"], [f"{fx(q[0][0])} {fx(q[0][1])} {fx(q[1][0])} {fx(q[1][1])}" for q in pairs])
    out = []
    for i, (q, r, az) in enumerate(zip(pairs, res, azis), 1):
        x, y, c, k = r
        if c != 0:
            continue
        lat, lon = ll_of(q[0], x, az[0])
        inp = {"a_start_lat": q[0][0], "a_start_lon": q[0][1], "a_end_lat": q[1][0], "a_end_lon": q[1][1],
               "b_start_lat": q[2][0], "b_start_lon": q[2][1], "b_end_lat": q[3][0], "b_end_lon": q[3][1]}
        exp = {"result.along_a.value": x, "result.along_b.value": y, "result.lat.value": lat, "result.lon.value": lon,
               "result.within": "yes" if k == 0 else "no", "ok": True}
        # Far out on shallow-angle extensions both solvers are limited by rounding.
        tol = {"result.along_a.value": {"abs": 1e-6, "rel": 5e-12}, "result.along_b.value": {"abs": 1e-6, "rel": 5e-12},
               "result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC_I, "sourceVersion": VER, "tolerance": tol})
    return out


def vertex_vectors():
    cases = [(40.6413, -73.7781, 51.47, -0.4543), (-33.9399, 151.1753, 37.6213, -122.379), (10, 0, None, 45.0),
             (-20, 30, None, 120.0), (60, 10, None, 270.0), (0, 0, None, 1.0), (35.0, 139.0, 40.0, -74.0)]
    out = []
    for lat1, lon1, lat2, other in cases:
        if lat2 is None:
            azi, s_len = other, None
            inp = {"lat1": lat1, "lon1": lon1, "azimuth": f"{azi} deg"}
        else:
            inv = run(["GeodSolve", "-i", "-p", "12"], [f"{fx(lat1)} {fx(lon1)} {fx(lat2)} {fx(other)}"])[0]
            azi, s_len = inv[0], inv[2]
            inp = {"lat1": lat1, "lon1": lon1, "lat2": lat2, "lon2": other}
        beta1 = math.atan((1 - F) * math.tan(math.radians(lat1)))
        sa, ca = math.sin(math.radians(azi)), math.cos(math.radians(azi))
        sigma1 = math.atan2(math.sin(beta1), ca * math.cos(beta1))
        arc = ((90 - math.degrees(sigma1)) + 180) % 360 - 180
        full = run(["GeodSolve", "-a", "-f", "-p", "12"], [f"{fx(lat1)} {fx(lon1)} {fx(azi)} {fx(arc)}"])[0]
        vlat, vlon, vazi, s12 = full[3], full[4], full[5], full[6]
        # Independent checks: due east or west there, and cos β_v = |sin α₀|.
        assert abs(abs(vazi) - 90) < 1e-9, vazi
        salp0 = sa * math.cos(beta1)
        beta_v = math.acos(min(1.0, abs(salp0)))
        assert abs(math.degrees(math.atan(math.tan(beta_v) / (1 - F))) - vlat) < 1e-9, (vlat, beta_v)
        exp = {"result.vertex_lat.value": vlat, "result.vertex_lon.value": vlon, "result.along.value": s12, "ok": True}
        if s_len is not None:
            exp["result.within"] = "yes" if 0 <= s12 <= s_len else "no"
        tol = {k: ({"abs": 1e-6} if "along" in k else {"abs": 1e-10}) for k, v in exp.items() if isinstance(v, float)}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC_V, "sourceVersion": VER, "tolerance": tol})
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for tool, vs in {"navigation.geodesic.intersection": seg_vectors(), "navigation.geodesic.vertex": vertex_vectors()}.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
        print(tool, len(vs))


if __name__ == "__main__":
    main()
