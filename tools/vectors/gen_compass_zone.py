#!/usr/bin/env python3
"""Golden vectors for geodesy.magnetic.uncertainty-zone. The field comes from
pygeomag 1.1.0 (WMM2025, run from a virtualenv with `pip install pygeomag`),
not from the core's spherical-harmonic code; the zones and the declination
uncertainty are the WMM2025 report's definitions written out here."""
import datetime as dt
import json
import math
import random
import sys
from pathlib import Path

import pygeomag

GEO = pygeomag.GeoMag()
SRC = "pygeomag 1.1.0 (WMM2025), with the WMM2025 report's blackout and caution zones and declination error model (tools/vectors/gen_compass_zone.py)"
VER = "WMM2025"


def decimal_year(date):
    d = dt.date.fromisoformat(date)
    start = dt.date(d.year, 1, 1)
    days = (dt.date(d.year + 1, 1, 1) - start).days
    return d.year + (d - start).days / days


def zone(h):
    return "blackout" if h < 2000 else "caution" if h < 6000 else "normal"


def main():
    rng = random.Random(99201)
    cases = [(74.697, -94.83, "2026-07-01", None), (40.015, -105.2705, "2026-09-18", "1655 m"), (-77.85, 166.67, "2027-01-15", None),
             (80.0, -72.0, "2025-06-01", None), (86.5, 140.0, "2029-12-31", None), (-64.0, 137.0, "2028-03-03", None)]
    while len(cases) < 20:
        lat = rng.choice([rng.uniform(-89, 89), rng.uniform(70, 89), rng.uniform(-89, -60)])
        cases.append((round(lat, 4), round(rng.uniform(-180, 180), 4), f"{rng.randint(2025, 2029)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}", None))
    out = []
    for i, (lat, lon, date, height) in enumerate(cases, 1):
        alt_km = float(height.split()[0]) / 1000 if height else 0.0
        r = GEO.calculate(glat=lat, glon=lon, alt=alt_km, time=decimal_year(date))
        inp = {"lat": lat, "lon": lon, "date": date}
        if height:
            inp["height"] = height
        exp = {"ok": True, "result.compass_zone": zone(r.h), "result.horizontal_intensity": r.h,
               "result.declination_uncertainty.value": min(180.0, math.sqrt(0.26 ** 2 + (5417 / r.h) ** 2)), "result.declination.value": r.d}
        # The two implementations agree within 5e-8° and 1e-6 nT; the geodesy angle ceiling is 1e-6°.
        tol = {"result.horizontal_intensity": {"rel": 1e-9, "abs": 1e-6}, "result.declination_uncertainty.value": {"abs": 1e-6},
               "result.declination.value": {"abs": 1e-6}}
        if abs(r.h - 2000) < 5 or abs(r.h - 6000) < 5:
            continue  # too near a zone edge to pin the zone across two implementations
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    for inp, code, field in [({"lat": 40, "lon": -105, "date": "2031-01-01"}, "OUT_OF_DOMAIN", "/date"), ({"lat": 40, "lon": -105, "date": "not a date"}, "INVALID_INPUT", "/date")]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
                    "source": "add-geodesy-suite scenarios", "sourceVersion": "2026", "tolerance": {}})
    (Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors") / "geodesy.magnetic.uncertainty-zone.jsonl").write_text(
        "".join(json.dumps(v, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
