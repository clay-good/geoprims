#!/usr/bin/env python3
"""Differential fixture for the GARS, GEOREF, and Maidenhead tools from
separately written Python libraries (run with them installed):

  pip install pygeodesy==26.9.9 maidenhead==1.8.0
  python tools/vectors/gen_gridref_diff.py

Writes core/crates/gp-geodesy/tests/data/gridref_diff.jsonl: one record per
point with pygeodesy's GARS (3 precisions) and GEOREF references (wgrs
levels 0, 1, and 3 to 7: 15°, 1°, and 1' to 0.0001'; its level 2, 10',
is not a GEOREF precision GeographicLib or the tool offers), their decoded
south-west corners and centers, and the maidenhead package's locators (4
to 10 characters) with their corners and centers. maidenhead 1.8.0 gets 2
characters wrong (CA for every latitude at 103°W) and wraps latitude 90°
to the south, so those are left out. It also rounds the last pair
instead of truncating, so every locator is checked in exact rational
arithmetic on the input's binary value; where the two differ the exact
locator is kept and the count goes in the header. One point in ten sits on
a cell edge or a pole. Seeded, so rerunning reproduces the file.
"""
import json
import random
from fractions import Fraction
from importlib.metadata import version
from pathlib import Path

import maidenhead
from pygeodesy import gars, wgrs

OUT = Path(__file__).resolve().parents[2] / "core/crates/gp-geodesy/tests/data/gridref_diff.jsonl"


def exact_maiden(lat, lon, chars):
    """The locator by truncation in exact arithmetic: fields of 20°×10°, then
    alternately 10 and 24 subdivisions. A value within a millionth of a
    finest cell of a grid line (the double nearest 70°50′, say) is taken as
    on the line, so it falls in the cell east or north of it."""
    def snap(v, per):
        k = round(v * per)
        return Fraction(k, per) if abs(v * per - k) < Fraction(1, 10**6) else v
    x = snap(Fraction(lon) + 180, 2880)
    y = min(snap(Fraction(lat) + 90, 5760), Fraction(180) - Fraction(1, 10**12))
    out, w, h = "", Fraction(20), Fraction(10)
    for i in range(chars // 2):
        n = 18 if i == 0 else (10 if i % 2 else 24)
        if i:
            w, h = w / n, h / n
        a, b = int(x // w) % n, int(y // h) % n
        x, y = x - (x // w) * w, y - (y // h) * h
        base = "0" if i % 2 else ("A" if i != 2 else "a")
        out += chr(ord(base) + a) + chr(ord(base) + b)
    return out


def point(rnd, i):
    if i % 10 == 0:
        # On a 5-minute grid line, or at a pole.
        return rnd.choice([90.0, -90.0, rnd.randint(-1079, 1079) / 12]), rnd.randint(-2160, 2159) / 12
    return rnd.uniform(-90, 90), rnd.uniform(-180, 180)


def main():
    rnd = random.Random(31)
    lines, fixed = [], 0
    for i in range(500):
        lat, lon = point(rnd, i)
        r = {"lat": lat, "lon": lon, "gars": [], "georef": [], "maidenhead": []}
        for p in range(3):
            g = gars.encode(lat, lon, precision=p)
            r["gars"].append([g, *gars.decode3(g, center=False)[:2], *gars.decode3(g)[:2]])
        for p in [0, 1, 3, 4, 5, 6, 7]:
            g = wgrs.encode(lat, lon, precision=p)
            r["georef"].append([g, *wgrs.decode3(g, center=False)[:2], *wgrs.decode3(g)[:2]])
        for p in range(2, 6) if lat < 90 else []:
            m = maidenhead.to_maiden(lat, lon, precision=p)
            e = exact_maiden(lat, lon, 2 * p)
            if m.upper() != e.upper():
                fixed, m = fixed + 1, e
            r["maidenhead"].append([m, *maidenhead.to_location(m), *maidenhead.to_location(m, center=True)])
        lines.append(json.dumps(r))
    head = json.dumps({"comment": f"pygeodesy {version('pygeodesy')}, maidenhead {version('maidenhead')}; seed 31; "
                                  f"{fixed} maidenhead locators replaced by the exact truncation"})
    OUT.write_text("\n".join([head] + lines) + "\n")


if __name__ == "__main__":
    main()
