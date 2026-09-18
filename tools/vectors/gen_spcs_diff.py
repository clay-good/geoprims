#!/usr/bin/env python3
"""SPCS83 differential fixture: 20 seeded random points inside each zone's
area of use, projected by PROJ (via pyproj) from NAD83 geographic (EPSG:4269)
to the zone in meters, with PROJ's meridian convergence and scale factor.
Writes core/crates/gp-geodesy/tests/data/spcs83_diff.csv. Needs pyproj."""
import random
import re
from pathlib import Path

import pyproj
from pyproj.database import query_crs_info

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/crates/gp-geodesy/tests/data/spcs83_diff.csv"
FIPS = {"NAD83 / Kentucky North": "1601"}


def main():
    rng = random.Random(83)
    lines = ["fips,lat,lon,easting,northing,convergence,scale"]
    for i in sorted(query_crs_info(auth_name="EPSG", pj_types=["PROJECTED_CRS"]), key=lambda i: int(i.code) if i.code.isdigit() else 0):
        if i.deprecated or not i.name.startswith("NAD83 / "):
            continue
        crs = pyproj.CRS.from_epsg(int(i.code))
        op = crs.coordinate_operation
        if op is None or not re.match(r"^SPCS83 .*\(meters?\)$", op.name):
            continue
        code = str(op.to_json_dict()["id"]["code"])[1:]
        fips = FIPS.get(i.name, code[:2] + code[3:].zfill(2))
        w, s, e, n = crs.area_of_use.bounds
        tr = pyproj.Transformer.from_crs(4269, crs, always_xy=True)
        proj = pyproj.Proj(crs)
        for _ in range(20):
            lat, lon = rng.uniform(s, n), rng.uniform(w, e)
            x, y = tr.transform(lon, lat)
            f = proj.get_factors(lon, lat)
            lines.append(f"{fips},{lat!r},{lon!r},{x!r},{y!r},{f.meridian_convergence!r},{f.meridional_scale!r}")
    OUT.write_text("\n".join(lines) + "\n")
    print(len(lines) - 1, "points")
    vectors(lines[1:])


SRC = "PROJ 9 (via pyproj) from EPSG:4269 to the zone's EPSG meter CRS (tools/vectors/gen_spcs_diff.py)"


def vec(i, inp, exp, tol):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": f"PROJ {pyproj.proj_version_str}",
            "tolerance": {k: {"abs": tol[k]} for k in exp}}


def vectors(rows):
    import json
    picks = [r.split(",") for r in rows[::400]]  # one point from every 20th zone
    fwd, inv = [], []
    for i, f in enumerate(picks, 1):
        fips, lat, lon, x, y = f[0], float(f[1]), float(f[2]), float(f[3]), float(f[4])
        fwd.append(vec(i, {"lat": lat, "lon": lon, "zone": fips, "unit": "m"},
                       {"result.easting.value": x, "result.northing.value": y}, {"result.easting.value": 1e-3, "result.northing.value": 1e-3}))
        inv.append(vec(i, {"zone": fips, "easting": f"{x!r} m", "northing": f"{y!r} m", "unit": "m"},
                       {"result.lat.value": lat, "result.lon.value": lon}, {"result.lat.value": 1e-9, "result.lon.value": 1e-9}))
    look = [({"query": "Colorado"}, 3), ({"query": "Texas"}, 5), ({"query": "Alaska"}, 10), ({"query": "Hawaii"}, 5), ({"query": "California"}, 6), ({"query": "Kentucky"}, 3)]
    lk = [{"id": f"v{i:03d}", "input": inp, "expect": {"ok": True, "result.count": n}, "source": "SPCS83 zone list, NOAA Manual NOS NGS 5",
           "sourceVersion": "NOS NGS 5 (1990)", "tolerance": {"result.count": {"abs": 0}}} for i, (inp, n) in enumerate(look, 1)]
    for name, vs in [("geodesy.spcs.spcs83-forward", fwd), ("geodesy.spcs.spcs83-inverse", inv), ("geodesy.spcs.zone-lookup", lk)]:
        (ROOT / "core/vectors" / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
