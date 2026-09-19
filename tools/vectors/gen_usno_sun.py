#!/usr/bin/env python3
"""USNO differential fixture for time.sun.events (standard library only):

  python3 tools/vectors/gen_usno_sun.py            # usno_sun.csv
  python3 tools/vectors/gen_usno_sun.py --nights   # usno_nights.csv

Queries the USNO Astronomical Applications API (rstt/oneday) for seeded
places and dates and writes core/crates/gp-time/tests/data/usno_sun.csv:
lat, lon, date, whole-hour offset, then USNO's rise, set, begin and end
civil twilight as local HH:MM ("-" when USNO lists none that day). The
offset is the nearest whole hour to the longitude, so each local day holds
one solar day. Pauses between requests to be polite to the service.

It also writes usno_nights.csv for time.sun.aviation-nights: 100 places,
mostly in the US and Alaska, with USNO's sunset and end of civil twilight
on the date and sunrise and beginning of civil twilight the next morning.
"""
import json
import random
import sys
import time
import urllib.request
from pathlib import Path

N = 300
API = "https://aa.usno.navy.mil/api/rstt/oneday?date={d}&coords={la:.4f},{lo:.4f}&tz={tz}"
PHEN = ["Rise", "Set", "Begin Civil Twilight", "End Civil Twilight"]


def fetch(d, la, lo, tz):
    with urllib.request.urlopen(API.format(d=d, la=la, lo=lo, tz=tz), timeout=30) as r:
        doc = json.load(r)
    time.sleep(0.25)
    return doc["apiversion"], {s["phen"]: s["time"] for s in doc["properties"]["data"]["sundata"]}


def nights():
    rnd = random.Random(41)
    rows, version = [], None
    while len(rows) < 100:
        k = len(rows)
        if k % 5 == 4:
            la, lo = rnd.uniform(55, 70), rnd.uniform(-165, -135)  # Alaska
        elif k % 5 == 3:
            la, lo = rnd.uniform(-45, 60), rnd.uniform(-180, 180)
        else:
            la, lo = rnd.uniform(25, 49), rnd.uniform(-124, -67)
        m, day = rnd.randint(1, 12), rnd.randint(1, 27)
        d, d2 = f"2026-{m:02d}-{day:02d}", f"2026-{m:02d}-{day + 1:02d}"
        tz = max(-12, min(12, round(lo / 15)))
        version, a = fetch(d, la, lo, tz)
        _, b = fetch(d2, la, lo, tz)
        rows.append(f"{la:.4f},{lo:.4f},{d},{tz},{a.get('Set', '-')},{a.get('End Civil Twilight', '-')},"
                    f"{b.get('Rise', '-')},{b.get('Begin Civil Twilight', '-')}")
    head = (f"# USNO API {version} rstt/oneday, retrieved {time.strftime('%Y-%m-%d')}; seed 41; "
            "lat,lon,date,offset_hours,set,civil_end,next_rise,next_civil_begin")
    Path("core/crates/gp-time/tests/data/usno_nights.csv").write_text("\n".join([head] + rows) + "\n")


def main():
    if "--nights" in sys.argv:
        return nights()
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
