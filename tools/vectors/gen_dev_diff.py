#!/usr/bin/env python3
"""Differential fixtures for the developer hero tools, from separately
written reference libraries (run with them installed):

  pip install pygeohash==3.3.2 mercantile==1.2.1 h3==4.4.2 geographiclib==2.1

  core/crates/gp-indexing/tests/data/geohash_diff.csv  pygeohash encode, decode_exactly
  core/crates/gp-indexing/tests/data/tile_diff.csv     mercantile tile, quadkey, bounds
  core/crates/gp-indexing/tests/data/h3_disk_diff.csv  H3 C grid_disk via h3-py
  core/crates/gp-navigation/tests/data/haversine_diff.csv  haversine in Python, and geographiclib Inverse for the ellipsoidal distance

Seeded, so rerunning reproduces the files.
"""
import math
import random
import sys
from importlib.metadata import version
from pathlib import Path

import h3
import mercantile
import pygeohash
from geographiclib.geodesic import Geodesic

N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
IDX = Path("core/crates/gp-indexing/tests/data")
NAV = Path("core/crates/gp-navigation/tests/data")


def geohash(rnd):
    rows = [f"# pygeohash {version('pygeohash')}; seed 11; lat,lon,precision,geohash,south,west,north,east"]
    for _ in range(N):
        la, lo, p = rnd.uniform(-90, 90), rnd.uniform(-180, 180), rnd.randint(1, 12)
        g = pygeohash.encode(la, lo, precision=p)
        clat, clon, elat, elon = pygeohash.decode_exactly(g)
        rows.append(f"{la!r},{lo!r},{p},{g},{clat - elat!r},{clon - elon!r},{clat + elat!r},{clon + elon!r}")
    return rows


def tiles(rnd):
    rows = [f"# mercantile {mercantile.__version__}; seed 12; lat,lon,zoom,x,y,quadkey,west,south,east,north"]
    for _ in range(N):
        la, lo, z = rnd.uniform(-85.05, 85.05), rnd.uniform(-180, 179.999999), rnd.randint(0, 24)
        t = mercantile.tile(lo, la, z)
        b = mercantile.bounds(t)
        rows.append(f"{la!r},{lo!r},{z},{t.x},{t.y},{mercantile.quadkey(t)},{b.west!r},{b.south!r},{b.east!r},{b.north!r}")
    return rows


def disks(rnd):
    rows = [f"# H3 C {h3.versions()['c']} via h3-py {h3.__version__}; seed 13; cell,k,cells in H3 C order separated by spaces"]
    for i in range(N // 4):
        if i % 10 == 0:
            r = rnd.randint(0, 15)
            c = h3.get_pentagons(r)[rnd.randint(0, 11)]
        else:
            c = h3.latlng_to_cell(rnd.uniform(-89.9, 89.9), rnd.uniform(-180, 180), rnd.randint(0, 15))
        k = rnd.randint(0, 6)
        rows.append(f"{c},{k},{' '.join(h3.grid_disk(c, k))}")
    return rows


def haversine(rnd):
    g = Geodesic.WGS84
    rows = [f"# geographiclib {version('geographiclib')}; seed 14; lat1,lon1,lat2,lon2,haversine_m,karney_m"]
    for _ in range(N):
        a = (rnd.uniform(-90, 90), rnd.uniform(-180, 180))
        # Mix short, regional, and global lines.
        s = rnd.choice([1e-3, 1, 30, 180])
        b = (max(-90, min(90, a[0] + rnd.uniform(-s, s))), (a[1] + rnd.uniform(-2 * s, 2 * s) + 180) % 360 - 180)
        p1, p2 = math.radians(a[0]), math.radians(b[0])
        h = math.sin((p2 - p1) / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(math.radians(b[1] - a[1]) / 2) ** 2
        hav = 2 * 6371008.771 * math.asin(math.sqrt(min(1.0, h)))
        rows.append(f"{a[0]!r},{a[1]!r},{b[0]!r},{b[1]!r},{hav!r},{g.Inverse(a[0], a[1], b[0], b[1])['s12']!r}")
    return rows


def main():
    for path, fn, seed in [(IDX / "geohash_diff.csv", geohash, 11), (IDX / "tile_diff.csv", tiles, 12),
                           (IDX / "h3_disk_diff.csv", disks, 13), (NAV / "haversine_diff.csv", haversine, 14)]:
        path.write_text("\n".join(fn(random.Random(seed))) + "\n")


if __name__ == "__main__":
    main()
