#!/usr/bin/env python3
"""Reads cases as JSON on stdin ([{date, points: [[level, lat, lon], ...]}]) and
prints, per case, how far pygeomag's declination at each point is from the
isogonic line the point is on: [worst, 95th percentile, median] in degrees."""
import datetime as dt
import json
import sys

import pygeomag

GEO = pygeomag.GeoMag()
out = []
for case in json.load(sys.stdin):
    d = dt.date.fromisoformat(case["date"])
    start, end = dt.date(d.year, 1, 1), dt.date(d.year + 1, 1, 1)
    t = d.year + (d - start).days / (end - start).days
    errs = sorted(abs((GEO.calculate(glat=lat, glon=lon, alt=0, time=t).d - level + 180) % 360 - 180) for level, lat, lon in case["points"])
    out.append([errs[-1], errs[int(len(errs) * 0.95)], errs[len(errs) // 2]])
print(json.dumps(out))
