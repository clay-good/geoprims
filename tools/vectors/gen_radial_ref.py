#!/usr/bin/env python3
"""Golden vectors for aviation.ifr.radial-fix from two independent libraries:
GeographicLib for Python (Karney) places the fix by the geodesic direct problem
from the station along the true course, and pygeomag (a separate WMM2025
implementation) gives the declination at the station that the tool compares
with the station's own variation.

Stations are placed on every continent and both sides of the antimeridian;
none has a variation within 0.05 deg of the 1 deg warning line.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_radial_ref.py   (with geographiclib and pygeomag installed)
"""
import datetime as dt
import json
from importlib.metadata import version
from pathlib import Path

import pygeomag
from geographiclib.geodesic import Geodesic

SRC = f"GeographicLib for Python {version('geographiclib')} (Geodesic.WGS84.Direct) and pygeomag {version('pygeomag')} (WMM2025) (tools/vectors/gen_radial_ref.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.ifr.radial-fix.jsonl"
GEO = pygeomag.GeoMag()
NM = 1852.0


def decimal_year(date):
    d = dt.date.fromisoformat(date)
    start, end = dt.date(d.year, 1, 1), dt.date(d.year + 1, 1, 1)
    return d.year + (d - start).days / (end - start).days


rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "pygeomag" not in r["source"]]
# station lat, lon, radial, distance NM, the station's published-style variation, date
CASES = [(39.8, -104.7, 98, 12.5, 11, "2026-09-22"), (51.47, -0.45, 250, 20, 0, "2027-01-15"), (-33.95, 151.18, 15, 40, 12, "2026-03-01"),
         (35.55, 139.78, 320, 8, -8, "2028-06-30"), (64.13, -21.94, 180, 60, -11, "2025-11-11"), (-34.82, -58.54, 75, 25, -9, "2029-02-02"),
         (1.36, 103.99, 200, 15, 0, "2026-12-25"), (61.17, -149.99, 5, 100, 15, "2027-08-08"), (-17.55, -149.61, 330, 30, 13, "2025-05-05"),
         (-36.85, 174.77, 90, 300, 21, "2026-07-04"), (25.25, 55.36, 135, 45, 2, "2028-10-10"), (19.44, -99.07, 270, 10, 6, "2029-09-09"),
         (-26.14, 28.25, 60, 5, -20, "2026-01-31"), (71.29, -156.77, 110, 80, 12, "2027-04-04")]
for lat, lon, radial, nm, var, date in CASES:
    tc = (radial + var) % 360
    fix = Geodesic.WGS84.Direct(lat, lon, tc, nm * NM)
    d = GEO.calculate(glat=lat, glon=lon, alt=0, time=decimal_year(date)).d
    diff = var - d
    assert abs(abs(diff) - 1) > 0.05, (lat, lon, diff)
    exp = {"ok": True, "result.fix_lat.value": fix["lat2"], "result.fix_lon.value": (fix["lon2"] + 540) % 360 - 180, "result.true_course.value": tc,
           "result.wmm_declination.value": d, "result.variation_difference.value": diff}
    tol = {"result.fix_lat.value": {"abs": 1e-9}, "result.fix_lon.value": {"abs": 1e-9}, "result.true_course.value": {"abs": 1e-9},
           "result.wmm_declination.value": {"abs": 1e-6}, "result.variation_difference.value": {"abs": 1e-6}}
    if abs(diff) > 1:
        exp["meta.warnings.*.code"] = "STATION_VARIATION_DIFFERS"
    rows.append({"id": f"v{len(rows) + 1:03d}", "input": {"lat": lat, "lon": lon, "radial": f"{radial} deg", "distance": f"{nm} NM", "variation": f"{var} deg", "date": date},
                 "expect": exp, "source": SRC, "sourceVersion": f"geographiclib {version('geographiclib')}, pygeomag {version('pygeomag')}", "tolerance": tol})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
