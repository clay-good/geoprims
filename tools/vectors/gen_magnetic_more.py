#!/usr/bin/env python3
"""Golden vectors for the two derived magnetic tools, from two outside models.

Both tools are one subtraction on top of the World Magnetic Model, and the
subtraction is the part people get backwards. The references here are
independent of the core in both halves:

- the declination D comes from `pygeomag`, a separate Python implementation of
  WMM2025. At Pittsburgh on 2026-07-02 it gives -9.2389527 and the core gives
  -9.2389552, which is two implementations of the same spherical harmonic
  expansion agreeing to three millionths of a degree.
- the grid convergence gamma comes from PROJ, through `Proj.get_factors`, which
  reports the meridian convergence of the projection at a point. For UTM zone
  17 at the same place PROJ gives 0.6603064307551 and the core 0.6603064306784.

Then grivation G = D - gamma, and magnetic = true - variation, both east
positive. Those two lines are the tools.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_magnetic_more.py
"""
import datetime as dt
import json
import sys
from pathlib import Path

import pygeomag
from pyproj import Proj

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = ("Declination from pygeomag (an independent WMM2025 implementation) and grid convergence from "
       "PROJ's Proj.get_factors; grivation G = D - gamma and magnetic = true - variation, east positive")
VER = "pygeomag / PROJ 9.3.0"
GEO = pygeomag.GeoMag()


def decimal_year(date):
    d = dt.date.fromisoformat(date)
    start = dt.date(d.year, 1, 1)
    end = dt.date(d.year + 1, 1, 1)
    return d.year + (d - start).days / (end - start).days


def declination(lat, lon, date):
    return GEO.calculate(glat=lat, glon=lon, alt=0, time=decimal_year(date)).d


def utm_zone(lat, lon):
    """The plain rule plus the two exceptions, which is all these cases need."""
    z = int((lon + 180) // 6) + 1
    if 56.0 <= lat < 64.0 and 3.0 <= lon < 12.0:
        z = 32
    if 72.0 <= lat < 84.0:
        for lo, hi, zz in ((0.0, 9.0, 31), (9.0, 21.0, 33), (21.0, 33.0, 35), (33.0, 42.0, 37)):
            if lo <= lon < hi:
                z = zz
    return z


def convergence(lat, lon):
    z = utm_zone(lat, lon)
    p = Proj(proj="utm", zone=z, ellps="WGS84", south=lat < 0)
    return p.get_factors(lon, lat).meridian_convergence


# (lat, lon, date) -- both hemispheres, either side of the agonic line, a
# Norway and a Svalbard point where the zone exception moves the convergence,
# and places where the declination is large.
GRIV_CASES = [
    (40.446111, -79.982222, "2026-07-02"),
    (51.5074, -0.1278, "2026-01-15"),
    (35.6762, 139.6503, "2026-03-21"),
    (-33.8688, 151.2093, "2026-09-18"),
    (-22.9068, -43.1729, "2026-06-01"),
    (60.0, 5.0, "2026-05-05"),
    (78.0, 15.0, "2026-05-05"),
    (64.1339, -21.8974, "2026-08-08"),
    (19.4326, -99.1332, "2026-02-14"),
    (1.3521, 103.8198, "2026-11-11"),
    (-45.0, 170.0, "2026-04-10"),
    (70.0, 25.0, "2026-12-21"),
    (25.2048, 55.2708, "2026-10-01"),
    (-1.2921, 36.8219, "2026-07-15"),
    (45.0, -123.0, "2027-01-01"),
    (-60.0, -60.0, "2026-05-20"),
]

# (bearing, direction, lat, lon, date)
T2M_CASES = [
    (0.0, "true-to-magnetic", 40.446111, -79.982222, "2026-07-02"),
    (90.0, "true-to-magnetic", 40.446111, -79.982222, "2026-07-02"),
    (180.0, "true-to-magnetic", 51.5074, -0.1278, "2026-01-15"),
    (270.0, "true-to-magnetic", 35.6762, 139.6503, "2026-03-21"),
    (45.0, "magnetic-to-true", 40.446111, -79.982222, "2026-07-02"),
    (315.0, "magnetic-to-true", -33.8688, 151.2093, "2026-09-18"),
    (5.0, "true-to-magnetic", 64.1339, -21.8974, "2026-08-08"),
    (355.0, "true-to-magnetic", 64.1339, -21.8974, "2026-08-08"),
    (120.0, "magnetic-to-true", 19.4326, -99.1332, "2026-02-14"),
    (240.0, "true-to-magnetic", 1.3521, 103.8198, "2026-11-11"),
    (359.0, "true-to-magnetic", -22.9068, -43.1729, "2026-06-01"),
    (1.0, "magnetic-to-true", -22.9068, -43.1729, "2026-06-01"),
    (30.0, "true-to-magnetic", 70.0, 25.0, "2026-12-21"),
    (210.0, "magnetic-to-true", -45.0, 170.0, "2026-04-10"),
    (60.0, "true-to-magnetic", 25.2048, 55.2708, "2026-10-01"),
    (300.0, "magnetic-to-true", -1.2921, 36.8219, "2026-07-15"),
]


def wrap360(x):
    return x % 360.0


def griv_rows(start):
    rows = []
    for i, (lat, lon, date) in enumerate(GRIV_CASES, start=start + 1):
        d = declination(lat, lon, date)
        g = convergence(lat, lon)
        rows.append((i, {"lat": lat, "lon": lon, "date": date},
                     {"ok": True, "result.grivation.value": d - g,
                      "result.declination.value": d, "result.convergence.value": g},
                     # Two WMM implementations agree to about 3e-6 deg; PROJ and
                     # the core's convergence to about 1e-9.
                     {"result.grivation.value": {"abs": 1e-4},
                      "result.declination.value": {"abs": 1e-4},
                      "result.convergence.value": {"abs": 1e-6}}))
    return rows


# (bearing, direction, chart variation, lat, lon, date) -- the side-by-side
# path, where a chart's printed variation is compared against the model.
CHART_CASES = [
    (0.0, "true-to-magnetic", "9.2W", 40.446111, -79.982222, "2026-07-02"),
    (90.0, "magnetic-to-true", "1E", 51.5074, -0.1278, "2026-01-15"),
    (270.0, "true-to-magnetic", "12E", 64.1339, -21.8974, "2026-08-08"),
    (180.0, "true-to-magnetic", "13E", -33.8688, 151.2093, "2026-09-18"),
]


def variation_degrees(text):
    """A chart variation like 9.2W, east positive."""
    value = float(text[:-1])
    return -value if text[-1].upper() == "W" else value


def t2m_rows(start):
    rows = []
    i = start
    for bearing, direction, lat, lon, date in T2M_CASES:
        i += 1
        d = declination(lat, lon, date)
        want = wrap360(bearing - d) if direction == "true-to-magnetic" else wrap360(bearing + d)
        # Without a chart variation the model's own declination is what the
        # tool used, and it is reported as `variation_used`; `model_declination`
        # only appears when there is a chart figure to set beside it.
        rows.append((i, {"bearing": bearing, "direction": direction,
                         "lat": lat, "lon": lon, "date": date},
                     {"ok": True, "result.result.value": want,
                      "result.variation_used.value": d,
                      "result.variation_source": "model"},
                     {"result.result.value": {"abs": 1e-4},
                      "result.variation_used.value": {"abs": 1e-4},
                      "result.variation_source": {"abs": 0}}))
    for bearing, direction, chart, lat, lon, date in CHART_CASES:
        i += 1
        d = declination(lat, lon, date)
        v = variation_degrees(chart)
        want = wrap360(bearing - v) if direction == "true-to-magnetic" else wrap360(bearing + v)
        model = wrap360(bearing - d) if direction == "true-to-magnetic" else wrap360(bearing + d)
        rows.append((i, {"bearing": bearing, "direction": direction, "variation": chart,
                         "lat": lat, "lon": lon, "date": date},
                     {"ok": True, "result.result.value": want,
                      "result.variation_used.value": v,
                      "result.variation_source": "chart",
                      "result.model_declination.value": d,
                      "result.model_result.value": model,
                      # Signed, and in this order: the chart figure minus the
                      # model's, so a positive difference means the chart reads
                      # further east than the model does.
                      "result.difference.value": v - d},
                     {"result.result.value": {"abs": 1e-9},
                      "result.variation_used.value": {"abs": 1e-9},
                      "result.variation_source": {"abs": 0},
                      "result.model_declination.value": {"abs": 1e-4},
                      "result.model_result.value": {"abs": 1e-4},
                      "result.difference.value": {"abs": 1e-4}}))
    return rows


def main():
    plan = [("geodesy.magnetic.grivation", griv_rows),
            ("geodesy.magnetic.true-to-magnetic", t2m_rows)]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 10:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": SRC, "sourceVersion": VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
