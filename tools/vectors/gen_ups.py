#!/usr/bin/env python3
"""Golden vectors for geodesy.ups.forward and geodesy.ups.inverse, from PROJ.

Universal Polar Stereographic is the polar half of the UTM/UPS pair: a polar
stereographic projection with k0 = 0.994 and a false easting and northing of
2,000,000 m, used north of 84 deg and south of 80 deg where UTM's zones become
useless. EPSG:32661 and EPSG:32761 are exactly that projection for the two
hemispheres, so pyproj gives the reference directly rather than by transcribing
a formula.

Both references agree at the anchor point: pyproj puts 85 deg N, 0 deg at
northing 1444542.6086173223 and GeographicLib's GeoConvert prints
1444542.6086, so the vectors are checked against two implementations that
share no code.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_ups.py
"""
import json
import math
import sys
from pathlib import Path

from pyproj import Transformer

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = ("PROJ through pyproj: EPSG:32661 and EPSG:32761, WGS 84 / UPS North and South "
       "(polar stereographic, k0 = 0.994, false easting and northing 2,000,000 m); "
       "the anchor point agrees with GeographicLib's GeoConvert")
VER = "pyproj 3.6.1 / PROJ 9.3.0"

FWD = Transformer.from_crs("EPSG:4326", "EPSG:32661", always_xy=True)
FWD_S = Transformer.from_crs("EPSG:4326", "EPSG:32761", always_xy=True)
INV = Transformer.from_crs("EPSG:32661", "EPSG:4326", always_xy=True)
INV_S = Transformer.from_crs("EPSG:32761", "EPSG:4326", always_xy=True)

# UPS covers 84 deg north and beyond, and 80 deg south and beyond. The cases
# walk the longitude circle at each limit, climb to the pole, and include the
# poles themselves, where longitude stops meaning anything.
NORTH = [
    (84.0, 0.0), (84.0, 45.0), (84.0, 90.0), (84.0, 135.0), (84.0, 180.0),
    (84.0, -45.0), (84.0, -90.0), (84.0, -135.0),
    (86.5, 30.0), (88.0, -120.0), (89.5, 60.0), (89.99, 170.0), (90.0, 0.0),
]
SOUTH = [
    (-80.0, 0.0), (-80.0, 45.0), (-80.0, 90.0), (-80.0, 180.0), (-80.0, -90.0),
    (-82.5, 15.0), (-85.0, -150.0), (-88.0, 100.0), (-89.9, -30.0), (-90.0, 0.0),
]


def forward_rows(start):
    rows = []
    i = start
    for lat, lon in NORTH + SOUTH:
        i += 1
        t = FWD if lat > 0 else FWD_S
        e, n = t.transform(lon, lat)
        expect = {"ok": True, "result.easting.value": e, "result.northing.value": n,
                  "result.hemisphere": "N" if lat > 0 else "S"}
        tol = {"result.easting.value": {"abs": 1e-4}, "result.northing.value": {"abs": 1e-4},
               "result.hemisphere": {"abs": 0}}
        # Convergence on a polar stereographic is the longitude itself, negated
        # in the south: the projection's grid north is the prime meridian. At
        # the antimeridian +180 and -180 are the same angle and an
        # implementation may report either, so that one case is left unpinned
        # rather than pinned to a spelling.
        if abs(lon) != 180.0:
            expect["result.convergence.value"] = lon if lat > 0 else -lon
            tol["result.convergence.value"] = {"abs": 1e-9}
        rows.append((i, {"lat": lat, "lon": lon}, expect, tol))
    return rows


def inverse_rows(start):
    rows = []
    i = start
    for lat, lon in NORTH + SOUTH:
        # The pole has no longitude to recover, so it is left to the forward
        # direction alone.
        if abs(lat) == 90.0:
            continue
        i += 1
        fwd = FWD if lat > 0 else FWD_S
        inv = INV if lat > 0 else INV_S
        e, n = fwd.transform(lon, lat)
        back_lon, back_lat = inv.transform(e, n)
        expect = {"ok": True, "result.lat.value": back_lat}
        tol = {"result.lat.value": {"abs": 1e-9}}
        if abs(lon) != 180.0:
            expect["result.lon.value"] = back_lon
            # Longitude is ill-conditioned near the pole: a fixed error on the
            # ground becomes an angle divided by cos(lat), so a metre at 89.99
            # deg is five thousand times the longitude it is at the equator.
            # The bound follows that factor instead of being one number that is
            # either too loose down here or too tight up there.
            tol["result.lon.value"] = {"abs": 1e-9 / math.cos(math.radians(lat))}
        rows.append((i, {"hemisphere": "N" if lat > 0 else "S",
                         "easting": f"{e:.6f} m", "northing": f"{n:.6f} m"}, expect, tol))
    return rows


def main():
    for tool, build in [("geodesy.ups.forward", forward_rows), ("geodesy.ups.inverse", inverse_rows)]:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 8:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": SRC, "sourceVersion": VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
