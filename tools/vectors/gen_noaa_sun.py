#!/usr/bin/env python3
"""Sunrise and sunset against the NOAA algorithm, as implemented in the astral
package (add-practitioner-essentials, "NOAA agreement": 1,000 random
locations and dates below 60° latitude, within 1 minute).

astral computes rise and set from NOAA's solar equations with the standard
0.833° allowance for refraction and the solar disc, written independently of
this tool. Events within half an hour of 00:00Z are skipped (see below). Each row is a place, a local date, a whole-hour offset near the
place's solar time (so the local day holds both events), and astral's
sunrise and sunset in local minutes after midnight, as fractions.

Writes core/crates/gp-time/tests/data/noaa_sun.csv. Requires astral."""
import datetime as dt
import random
from importlib.metadata import version
from pathlib import Path

from astral import Observer
from astral.sun import sunrise, sunset

OUT = Path("core/crates/gp-time/tests/data/noaa_sun.csv")


def main():
    rng = random.Random(1000)
    rows = []
    while len(rows) < 1000:
        lat = round(rng.uniform(-60, 60), 4)
        lon = round(rng.uniform(-180, 180), 4)
        day = dt.date(1950, 1, 1) + dt.timedelta(days=rng.randrange(0, 365 * 100))
        off = max(-12, min(14, round(lon / 15)))
        tz = dt.timezone(dt.timedelta(hours=off))
        obs = Observer(latitude=lat, longitude=lon, elevation=0)
        try:
            r, s = sunrise(obs, day, tzinfo=tz), sunset(obs, day, tzinfo=tz)
        except ValueError:
            continue
        # astral evaluates NOAA's equations for the UTC day, so an event
        # within half an hour of 00:00Z takes the neighboring day's
        # declination: at 54.8° S on 2049-04-07 its sunrise is 2 minutes
        # late against NREL SPA (06:58:47, the tool's 06:59). Such draws are
        # skipped; tests/noaa_sun.rs checks that one against SPA instead.
        near_midnight = lambda t: min(  # noqa: E731
            (t.astimezone(dt.timezone.utc) - t.astimezone(dt.timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)).total_seconds(),
            86400 - (t.astimezone(dt.timezone.utc) - t.astimezone(dt.timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)).total_seconds(),
        ) < 1800
        if near_midnight(r) or near_midnight(s):
            continue
        mins = lambda t: (t.date() - day).days * 1440 + t.hour * 60 + t.minute + (t.second + t.microsecond / 1e6) / 60  # noqa: E731
        rows.append(f"{lat},{lon},{day.isoformat()},{off},{mins(r):.4f},{mins(s):.4f}")
    OUT.write_text(f"# lat,lon,date,offset_hours,sunrise_min,sunset_min (astral {version('astral')}, NOAA equations)\n" + "\n".join(rows) + "\n")
    print(len(rows))


if __name__ == "__main__":
    main()
