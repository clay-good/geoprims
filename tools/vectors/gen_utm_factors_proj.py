#!/usr/bin/env python3
"""UTM convergence and point scale against PROJ (geodesy task 4.7, the UTM
half of "PROJ factors agreement"; the SPCS83 half is spcs83_diff.csv).

PROJ's +proj=utm (the extended transverse Mercator, Poder/Engsager) gives
each point's meridian convergence and meridional scale through
Proj.get_factors, which PROJ computes by numerical differentiation, good to
about 1e-7. 2,000 points in all 60 zones from 80 S to 84 N, each in its own
zone, within the zone's 6-degree strip and up to 3 degrees past it (as UTM
coordinates are used near zone edges).

Writes core/crates/gp-geodesy/tests/data/utm_factors_proj.csv. Run with the
Python that has pyproj."""
import csv
import random
from pathlib import Path

import pyproj

OUT = Path("core/crates/gp-geodesy/tests/data/utm_factors_proj.csv")


def main():
    rng = random.Random(47)
    rows = []
    while len(rows) < 2000:
        zone = rng.randint(1, 60)
        lat = rng.uniform(-80, 84)
        cm = -183 + 6 * zone
        lon = cm + rng.uniform(-3, 3) * (1.0 if rng.random() < 0.8 else 2.0)
        lon = (lon + 540) % 360 - 180
        south = lat < 0
        p = pyproj.Proj(proj="utm", zone=zone, ellps="WGS84", south=south)
        f = p.get_factors(lon, lat)
        rows.append([zone, "S" if south else "N", f"{lat:.12f}", f"{lon:.12f}", repr(f.meridian_convergence), repr(f.meridional_scale)])
    with OUT.open("w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow([f"# PROJ {pyproj.proj_version_str}, pyproj {pyproj.__version__}: zone, hemisphere, lat, lon, meridian convergence (deg), point scale"])
        w.writerows(rows)
    print(len(rows), "points")


if __name__ == "__main__":
    main()
