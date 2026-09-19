#!/usr/bin/env python3
"""MGRS differential fixture from NGA GEOTRANS (C), via the mgrs package:

  pip install mgrs==1.5.4 packaging pyproj==3.6.1
  python tools/vectors/gen_mgrs_diff.py

Writes core/crates/gp-geodesy/tests/data/mgrs_diff.csv: lat, lon, digits
per axis (0-5), the reference, and GEOTRANS's corner. One point in five
falls in the Norway and Svalbard exceptions and one in seven in the UPS
polar caps. Seeded, so rerunning reproduces the file.

GEOTRANS's projection series is off by up to 1.5 cm near the UPS edge and
far out in Svalbard's widened zones, which can flip a truncated digit when
a point is that close to a grid line. So the reference keeps GEOTRANS's
zone and square letters but takes its digits from PROJ's exact grid
coordinates; rows where that changes GEOTRANS's answer are counted in the
header.
"""
import random
from importlib.metadata import version
from pathlib import Path

import mgrs
import pyproj

N = 2000


def exact(g, la, lo, d):
    """g with its digits recomputed from PROJ's grid coordinates in g's zone."""
    head = g[: len(g) - 2 * d]
    if head[0].isdigit():
        zone = int(head[:-3])
        epsg = (32600 if head[-3] >= "N" else 32700) + zone
    else:
        epsg = 32661 if head[0] in "YZ" else 32761
    e, n = pyproj.Transformer.from_crs(4326, epsg, always_xy=True).transform(lo, la)
    if d == 0:
        return head
    step = 10 ** (5 - d)
    return f"{head}{int(e % 100000 // step):0{d}d}{int(n % 100000 // step):0{d}d}"


def main():
    m, rnd = mgrs.MGRS(), random.Random(21)
    rows, flipped = [], 0
    while len(rows) < N:
        i = len(rows) + 1
        if i % 5 == 0:
            la, lo = rnd.uniform(56, 84), rnd.uniform(0, 42)
        elif i % 7 == 0:
            la, lo = rnd.choice([rnd.uniform(-90, -80), rnd.uniform(84, 90)]), rnd.uniform(-180, 180)
        else:
            la, lo = rnd.uniform(-80, 84), rnd.uniform(-180, 180)
        d = rnd.randint(0, 5)
        g = m.toMGRS(la, lo, MGRSPrecision=d)
        ref = exact(g, la, lo, d)
        clat, clon = m.toLatLon(ref)
        flipped += ref != g
        rows.append(f"{la!r},{lo!r},{d},{ref},{clat!r},{clon!r}")
    head = (f"# GEOTRANS via mgrs {version('mgrs')}, digits from PROJ {pyproj.proj_version_str} (pyproj {pyproj.__version__}); seed 21; "
            f"{flipped} of {N} references differ from GEOTRANS by a flipped last digit; lat,lon,digits,mgrs,corner_lat,corner_lon")
    Path("core/crates/gp-geodesy/tests/data").mkdir(parents=True, exist_ok=True)
    Path("core/crates/gp-geodesy/tests/data/mgrs_diff.csv").write_text("\n".join([head] + rows) + "\n")


if __name__ == "__main__":
    main()
