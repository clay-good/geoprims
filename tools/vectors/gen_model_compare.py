#!/usr/bin/env python3
"""Golden vectors for geodesy.magnetic.model-compare. WMM2025 comes from
pygeomag 1.1.0 and IGRF-14 from ppigrf 2.1.0 (both `pip install`ed into a
virtualenv), not from the core's spherical-harmonic code."""
import datetime as dt
import json
import math
import random
import sys
from pathlib import Path

import numpy as np
import ppigrf
import pygeomag

GEO = pygeomag.GeoMag()
SRC = "pygeomag 1.1.0 (WMM2025) and ppigrf 2.1.0 (IGRF-14) (tools/vectors/gen_model_compare.py)"
VER = "WMM2025, IGRF-14"


def decimal_year(date):
    d = dt.date.fromisoformat(date)
    start = dt.date(d.year, 1, 1)
    return d.year + (d - start).days / (dt.date(d.year + 1, 1, 1) - start).days


def igrf(lat, lon, h_km, date):
    # ppigrf interpolates between 2025-01-01 and 2030-01-01 in elapsed time;
    # the models' decimal year counts each calendar year as one. Ask ppigrf
    # for the instant at the same fraction of its five-year span.
    t0, t1 = dt.datetime(2025, 1, 1), dt.datetime(2030, 1, 1)
    when = t0 + (t1 - t0) * ((decimal_year(date) - 2025) / 5)
    be, bn, bu = (float(np.ravel(c)[0]) for c in ppigrf.igrf(lon, lat, h_km, when))
    return math.degrees(math.atan2(be, bn)), math.degrees(math.atan2(-bu, math.hypot(be, bn))), math.sqrt(be * be + bn * bn + bu * bu)


def main():
    rng = random.Random(40417)
    cases = [(61.2181, -149.9003, "2027-01-01", None), (40.015, -105.2705, "2026-09-18", "1655 m"), (-33.8688, 151.2093, "2025-03-01", None),
             (51.4779, -0.0015, "2028-06-15", None), (-77.85, 166.67, "2029-11-30", "400 km")]
    while len(cases) < 22:
        cases.append((round(rng.uniform(-80, 80), 4), round(rng.uniform(-180, 180), 4),
                      f"{rng.randint(2025, 2029)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}", None))
    out = []
    for lat, lon, date, height in cases:
        h_km = 0.0
        if height:
            v, u = height.split()
            h_km = float(v) / (1000 if u == "m" else 1)
        r = GEO.calculate(glat=lat, glon=lon, alt=h_km, time=decimal_year(date))
        d, i, f = igrf(lat, lon, h_km, date)
        inp = {"lat": lat, "lon": lon, "date": date}
        if height:
            inp["height"] = height
        dd = (r.d - d + 540) % 360 - 180
        exp = {"ok": True, "result.declination_difference.value": dd, "result.wmm_declination.value": r.d,
               "result.igrf_declination.value": d, "result.inclination_difference.value": r.i - i, "result.intensity_difference": r.f - f}
        tol = {k: {"abs": 1e-6} for k in exp if k != "ok" and k != "result.intensity_difference"}
        tol["result.intensity_difference"] = {"abs": 1e-3}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    for inp, code, field in [({"lat": 40, "lon": -105, "date": "2024-06-01"}, "OUT_OF_DOMAIN", "/date"),
                             ({"lat": 40, "lon": -105, "date": "2027-01-01", "height": "900 km"}, "OUT_OF_DOMAIN", "/height")]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
                    "source": "add-geodesy-suite scenarios", "sourceVersion": "2026", "tolerance": {}})
    (Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors") / "geodesy.magnetic.model-compare.jsonl").write_text(
        "".join(json.dumps(v, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
