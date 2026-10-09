#!/usr/bin/env python3
"""A one-off differential check of survey.gnss.dop against an independent
orbit source: the core's azimuth and elevation of each GPS satellite, from the
Coast Guard's YUMA almanac, against Skyfield's SGP4 propagation of CelesTrak's
GPS two-line elements for the same week. Not run in the gates: it needs the
network for both files and Skyfield in a scratch virtualenv.

  curl -o current_yuma.alm https://www.navcen.uscg.gov/sites/default/files/gps/almanac/current_yuma.alm
  curl -o gps-ops.tle 'https://celestrak.org/NORAD/elements/gp.php?GROUP=gps-ops&FORMAT=tle'
  # core-sky.json: run survey.gnss.dop on that almanac with a 0° mask and a
  # 0 h window at each site, and keep result.satellites as [prn, azimuth, elevation]
  python3 tools/vectors/check_dop_skyfield.py core-sky.json gps-ops.tle

core-sky.json maps "lat,lon,startZ" to [[prn, azimuth, elevation], ...].
"""
import json
import re
import sys

from skyfield.api import EarthSatellite, load, wgs84

core = json.load(open(sys.argv[1]))
ts = load.timescale(builtin=True)
lines = open(sys.argv[2]).read().splitlines()
sats = {}
for i in range(0, len(lines) - 2, 3):
    m = re.search(r"PRN (\d+)", lines[i])
    if m:
        sats[int(m.group(1))] = EarthSatellite(lines[i + 1], lines[i + 2], lines[i].strip(), ts)
diffs = []
for key, rows in core.items():
    lat, lon, start = key.split(",", 2)
    t = ts.utc(int(start[:4]), int(start[5:7]), int(start[8:10]), int(start[11:13]), int(start[14:16]))
    site = wgs84.latlon(float(lat), float(lon))
    for prn, az, el in rows:
        if prn in sats:
            alt, azs, _ = (sats[prn] - site).at(t).altaz()
            diffs.append((abs(el - alt.degrees), abs((az - azs.degrees + 180) % 360 - 180), prn, key))
diffs.sort(reverse=True)
within = sum(d[0] < 0.1 for d in diffs)
print(f"{len(diffs)} sightings; {within} within 0.1° in elevation; worst: {diffs[:3]}")
