#!/usr/bin/env python3
"""Golden vectors for geodesy.utm.zone, from GeographicLib's GeoConvert.

The UTM zone of a point is `floor((lon + 180) / 6) + 1` everywhere except two
places, and the two exceptions are the whole difficulty:

- **Norway**: between 56 and 64 deg north, zone 32 is widened westward to 3 deg
  east, so a point at 3-6 deg east that the formula puts in zone 31 is really
  in zone 32.
- **Svalbard**: between 72 and 84 deg north, zones 31, 33, 35 and 37 are
  widened and 32, 34 and 36 disappear, so the boundaries fall at 9, 21 and 33
  deg east instead of every 6.

GeographicLib implements both, so `GeoConvert -u` gives the zone and
`GeoConvert -m` the MGRS grid zone whose first characters are the zone and the
latitude band. Those are read here rather than transcribed, and the central
meridian is computed from the zone by its own definition, 6z - 183.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_utm_zone.py
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = ("GeographicLib's GeoConvert: -u for the UTM zone and -m for the MGRS grid zone, "
       "both carrying the Norway and Svalbard exceptions; central meridian 6z - 183 by definition")
VER = "GeographicLib 2.7"

CASES = [
    # The plain formula, around the world and either side of the antimeridian.
    (0.0, 0.0), (0.0, 179.9), (0.0, -179.9), (0.0, -0.1), (0.0, 6.0),
    (-35.0, -70.0), (35.6762, 139.6503), (-33.8688, 151.2093), (1.3521, 103.8198),
    (51.5074, -0.1278), (19.4326, -99.1332), (-22.9068, -43.1729),
    # Norway: 56-64 N, 3-12 E. Inside, and a degree outside on each side.
    (56.0, 3.0), (56.0, 5.0), (60.0, 11.9), (63.9, 4.0),
    (55.9, 5.0), (64.1, 5.0), (60.0, 2.9), (60.0, 12.1),
    # Svalbard: 72-84 N, boundaries at 9, 21 and 33 E.
    (72.0, 7.0), (78.0, 10.0), (78.0, 20.0), (78.0, 22.0), (78.0, 34.0),
    (71.9, 10.0), (75.0, 0.0), (75.0, 40.0),
    # Exactly 84 deg north is left out on purpose. The core keeps it in
    # UTM, symmetric with the -80 deg edge it shares with GeographicLib;
    # GeographicLib switches to UPS there so that UTM and UPS tile without
    # overlapping. Both readings of "80 S to 84 N" are defensible, so the
    # difference is written into the limitations rather than pinned here.
    (83.999, 10.0),
    # The band letters at their edges, and the two the scheme leaves out.
    (-79.9, 20.0), (83.9, 20.0), (8.0, 20.0), (-8.0, 20.0),
]


def geoconvert(lat, lon, flag):
    out = subprocess.run(
        ["GeoConvert", flag, "-p", "0"],
        input=f"{lat} {lon}\n", capture_output=True, text=True, check=True,
    ).stdout.strip()
    return out


def zone_band(lat, lon):
    """(zone, band letter) from GeographicLib, or (0, letter) at the poles."""
    utm = geoconvert(lat, lon, "-u")
    mgrs = geoconvert(lat, lon, "-m")
    m = re.match(r"^(\d*)([A-Z])", mgrs)
    band = m.group(2)
    z = re.match(r"^(\d*)[ns]", utm)
    zone = int(z.group(1)) if z and z.group(1) else 0
    return zone, band


def main():
    path = ROOT / "geodesy.utm.zone.jsonl"
    existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start > 8:
        raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (lat, lon) in enumerate(CASES, start=start + 1):
        zone, band = zone_band(lat, lon)
        expect = {"ok": True, "result.zone": zone,
                  "result.grid_zone": f"{zone:02d}{band}" if zone else band}
        tol = {"result.zone": {"abs": 0}, "result.grid_zone": {"abs": 0}}
        if zone:
            # The central meridian of zone z, from the zone's own definition.
            expect["result.central_meridian.value"] = float(6 * zone - 183)
            tol["result.central_meridian.value"] = {"abs": 0}
        rows.append({"id": f"v{i:03d}", "input": {"lat": lat, "lon": lon},
                     "expect": expect, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    with path.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
