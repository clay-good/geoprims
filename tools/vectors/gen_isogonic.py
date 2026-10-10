#!/usr/bin/env python3
"""Golden vectors for geodesy.magnetic.isogonic from pygeomag 1.1.0, a
separate Python implementation of WMM2025 (pip install pygeomag).

A contour map cannot be predicted point for point without redoing the
contouring, so these vectors hold what an independent scan of the field fixes:
the lowest isogonic line a region holds. pygeomag's declination on a fine grid
gives the region's lowest value, and the first line is the first multiple of
the interval above it. Regions are chosen so that value is not within a tenth of the
interval of a multiple, where a coarser grid could fairly differ. Every point of every
line is checked against pygeomag in tools/diff/isogonic.test.mjs.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_isogonic.py   (with pygeomag installed)
"""
import datetime as dt
import json
import math
from importlib.metadata import version
from pathlib import Path

import pygeomag

VER = version("pygeomag")
SRC = f"pygeomag {VER} (independent WMM2025): the lowest declination on a 0.1 deg grid of the region, rounded up to the interval (tools/vectors/gen_isogonic.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geodesy.magnetic.isogonic.jsonl"
GEO = pygeomag.GeoMag()


def decimal_year(date):
    d = dt.date.fromisoformat(date)
    start, end = dt.date(d.year, 1, 1), dt.date(d.year + 1, 1, 1)
    return d.year + (d - start).days / (end - start).days


def lowest(south, west, north, east, date):
    t = decimal_year(date)
    lo = math.inf
    la = south
    while la <= north + 1e-9:
        lon = west
        while lon <= east + 1e-9:
            lo = min(lo, GEO.calculate(glat=la, glon=lon, alt=0, time=t).d)
            lon += 0.1
        la += 0.1
    return lo


rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if "pygeomag" not in r["source"]]
CASES = [("2026-09-18", 37, -109, 41, -102, 1), ("2026-09-18", 24, -125, 50, -66, 2), ("2027-03-01", 35, -10, 60, 30, 1), ("2025-06-15", -42, 112, -12, 152, 2),
         ("2028-11-30", -35, 10, 5, 45, 2), ("2026-01-01", 55, -170, 72, -130, 5), ("2026-06-01", 25, -100, 37, -80, 1), ("2029-12-01", 40, -80, 48, -66, 1),
         ("2025-01-15", 5, 65, 38, 98, 1), ("2027-07-04", -56, -76, -17, -52, 2), ("2026-03-20", 30, 128, 46, 146, 1), ("2028-02-29", -40, 165, -33, 179, 1),
         ("2026-09-18", 45, -125, 49, -116, 0.5), ("2026-12-31", -40, 115, -15, 150, 2)]
for date, s, w, n, e, step in CASES:
    lo = lowest(s, w, n, e, date)
    first = math.ceil(lo / step) * step
    # Keep clear of a multiple: there a coarser grid could fairly start a line later.
    gap = first - lo
    assert gap > 0.1 * step and step - gap > 0.1 * step, (date, s, w, lo, first)
    rows.append({"id": f"v{len(rows) + 1:03d}",
                 "input": {"date": date, "south": s, "west": w, "north": n, "east": e, "interval": f"{step} deg"},
                 "expect": {"ok": True, "result.interval.value": step, "result.lines.0.level.value": first},
                 "source": SRC, "sourceVersion": f"pygeomag {VER}",
                 "tolerance": {"result.interval.value": {"abs": 1e-12}, "result.lines.0.level.value": {"abs": 1e-9}}})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
