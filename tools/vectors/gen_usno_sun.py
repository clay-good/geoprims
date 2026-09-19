#!/usr/bin/env python3
"""USNO differential fixture for time.sun.events (standard library only):

  python3 tools/vectors/gen_usno_sun.py

Queries the USNO Astronomical Applications API (rstt/oneday) for seeded
places and dates and writes core/crates/gp-time/tests/data/usno_sun.csv:
lat, lon, date, whole-hour offset, then USNO's rise, set, begin and end
civil twilight as local HH:MM ("-" when USNO lists none that day). The
offset is the nearest whole hour to the longitude, so each local day holds
one solar day. Pauses between requests to be polite to the service.
"""
import json
import random
import time
import urllib.request
from pathlib import Path

N = 300
API = "https://aa.usno.navy.mil/api/rstt/oneday?date={d}&coords={la:.4f},{lo:.4f}&tz={tz}"
PHEN = ["Rise", "Set", "Begin Civil Twilight", "End Civil Twilight"]


def main():
    rnd = random.Random(31)
    rows = []
    version = None
    while len(rows) < N:
        k = len(rows)
        # Mostly the populated latitudes, some high-latitude and polar cases.
        la = rnd.uniform(-60, 65) if k % 6 else rnd.choice([rnd.uniform(62, 72), rnd.uniform(-72, -62)])
        lo = rnd.uniform(-180, 180)
        d = f"2026-{rnd.randint(1, 12):02d}-{rnd.randint(1, 28):02d}"
        tz = max(-12, min(12, round(lo / 15)))
        with urllib.request.urlopen(API.format(d=d, la=la, lo=lo, tz=tz), timeout=30) as r:
            doc = json.load(r)
        version = doc["apiversion"]
        sun = {s["phen"]: s["time"] for s in doc["properties"]["data"]["sundata"]}
        rows.append(f"{la:.4f},{lo:.4f},{d},{tz}," + ",".join(sun.get(p, "-") for p in PHEN))
        time.sleep(0.25)
    head = f"# USNO API {version} rstt/oneday, retrieved {time.strftime('%Y-%m-%d')}; seed 31; lat,lon,date,offset_hours,rise,set,civil_begin,civil_end"
    out = Path("core/crates/gp-time/tests/data/usno_sun.csv")
    out.write_text("\n".join([head] + rows) + "\n")


if __name__ == "__main__":
    main()
