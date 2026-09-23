#!/usr/bin/env python3
"""Solar-position differential fixture from pvlib's NREL SPA.

    gen_spa_diff.py OUT.csv COUNT SEED

Rows: lat,lon,unix,elevation_true,elevation_apparent,azimuth -- degrees.

The core implements the NREL Solar Position Algorithm; so does pvlib, from the
same published paper but as a separate piece of work. Comparing them checks the
*implementation* -- the dozens of periodic terms, the nutation series, the
refraction correction -- which is the part a transcription error lives in. The
*model* is checked elsewhere, against USNO's own published rise and set times
(tests/usno_sun.rs), because two SPA implementations agreeing says nothing
about whether SPA is right.

Places are drawn uniformly over the sphere by latitude-weighted sampling, so
the poles are not over-represented, and times uniformly over 1990-2060. Both
elevations are recorded: the geometric one, which no atmosphere touches, and
the apparent one at standard sea-level pressure and 12 degrees C, which is the
refraction model the core states.
"""
import random
import sys

import pandas as pd
import pvlib

EPOCH_LO = 631152000   # 1990-01-01
EPOCH_HI = 2871763200  # 2061-01-01


def main():
    out, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    rnd = random.Random(seed)
    rows = []
    for _ in range(count):
        # Uniform on the sphere: latitude by arcsine, not uniform in degrees.
        import math
        lat = math.degrees(math.asin(rnd.uniform(-1.0, 1.0)))
        lon = rnd.uniform(-180.0, 180.0)
        t = rnd.randint(EPOCH_LO, EPOCH_HI)
        ts = pd.DatetimeIndex([pd.Timestamp(t, unit="s", tz="UTC")])
        r = pvlib.solarposition.spa_python(
            ts, lat, lon, altitude=0, pressure=101325, temperature=12
        )
        rows.append(
            f"{lat:.9f},{lon:.9f},{t},"
            f"{float(r['elevation'].iloc[0]):.9f},"
            f"{float(r['apparent_elevation'].iloc[0]):.9f},"
            f"{float(r['azimuth'].iloc[0]):.9f}"
        )
    with open(out, "w") as f:
        f.write(f"# pvlib {pvlib.__version__} spa_python, sea level, 101325 Pa, 12 C; seed {seed}\n")
        f.write("\n".join(rows) + "\n")
    print(f"{len(rows)} rows -> {out}")


if __name__ == "__main__":
    main()
