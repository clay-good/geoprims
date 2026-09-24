#!/usr/bin/env python3
"""S2's own RegionCoverer on rectangles and caps, from s2sphere, for the port
in core/crates/gp-indexing/src/s2exact.rs (add-spatial-indexing-and-raster
task 2.2, "parity with S2's own choice of cells").

400 random regions: rectangles (some across the antimeridian, some reaching
a pole) and caps (from a few meters to thousands of kilometers), each with a
level range and a cell budget. The angle of a cap is the radius over
6,371,008.8 m, as indexing.s2.covering converts it.

Writes core/crates/gp-indexing/tests/data/s2_coverer_parity.jsonl."""
import json
import math
import random
import signal
from pathlib import Path

import s2sphere as s2

EARTH_M = 6_371_008.8
OUT = Path("core/crates/gp-indexing/tests/data/s2_coverer_parity.jsonl")


def _timeout(*_):
    raise TimeoutError


def main():
    signal.signal(signal.SIGALRM, _timeout)
    rng = random.Random(2002)
    rows = []
    while len(rows) < 400:
        lo = rng.randint(0, 14)
        hi = min(30, lo + rng.randint(0, 10))
        cells = rng.choice([1, 3, 4, 8, 12, 20, 50, 200])
        if rng.random() < 0.5:
            south = rng.uniform(-90, 89.9)
            north = min(90.0, south + 10 ** rng.uniform(-4, 1.5))
            west = rng.uniform(-180, 180)
            east = west + 10 ** rng.uniform(-4, 2.3)
            if east > 180:
                east -= 360
            region = s2.LatLngRect(s2.LatLng.from_degrees(south, west), s2.LatLng.from_degrees(north, east))
            params = {"kind": "rect", "south": south, "north": north, "west": west, "east": east}
        else:
            lat, lon = math.degrees(math.asin(rng.uniform(-1, 1))), rng.uniform(-180, 180)
            radius = 10 ** rng.uniform(0, 6.5)
            axis = s2.LatLng.from_degrees(lat, lon).to_point()
            region = s2.Cap.from_axis_angle(axis, s2.Angle.from_radians(radius / EARTH_M))
            params = {"kind": "cap", "lat": lat, "lon": lon, "radius_m": radius}
        cov = s2.RegionCoverer()
        cov.min_level, cov.max_level, cov.max_cells = lo, hi, cells
        # s2sphere is pure Python: a wide region with a fine level range can
        # take minutes, so such a draw is skipped rather than waited on.
        signal.alarm(20)
        try:
            got = cov.get_covering(region)
        except TimeoutError:
            continue
        finally:
            signal.alarm(0)
        if len(got) > 2000:
            continue
        rows.append({**params, "min_level": lo, "max_level": hi, "max_cells": cells, "cells": [c.to_token() for c in got]})
        if len(rows) % 50 == 0:
            print(len(rows), "regions", flush=True)
    OUT.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
    print(len(rows), "regions,", sum(len(r["cells"]) for r in rows), "cells")


if __name__ == "__main__":
    main()
