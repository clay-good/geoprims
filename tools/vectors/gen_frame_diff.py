#!/usr/bin/env python3
"""Differential fixtures for the ECEF and local-frame tools from GeographicLib's
CartConvert (C++, run with it on PATH):

  core/crates/gp-geodesy/tests/data/ecef_diff.csv  geodetic -> ECEF, and ECEF -> geodetic (CartConvert -r)
  core/crates/gp-geodesy/tests/data/enu_diff.csv   geodetic -> local ENU at random origins (CartConvert -l)

Seeded, so rerunning reproduces the files.
"""
import random
import subprocess
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/crates/gp-geodesy/tests/data"


def cart(args, lines):
    out = subprocess.run(["CartConvert", "-p", "9", *args], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout
    return [l.split() for l in out.splitlines() if l.strip()]


def version():
    return subprocess.run(["CartConvert", "--version"], capture_output=True, text=True).stdout.split()[-1]


def ecef(rnd, n=1000):
    # Heights from below the geoid to beyond geostationary orbit; one in ten at a pole.
    pts = []
    for i in range(n):
        lat = rnd.choice([90.0, -90.0]) if i % 10 == 0 else rnd.uniform(-90, 90)
        h = rnd.choice([0.0, rnd.uniform(-400, 9000), rnd.uniform(0, 1e6), rnd.uniform(0, 4e7)])
        pts.append((lat, rnd.uniform(-180, 180), h))
    fwd = cart([], [f"{a!r} {b!r} {c!r}" for a, b, c in pts])
    inv = cart(["-r"], [" ".join(r) for r in fwd])
    rows = [f"# GeographicLib CartConvert {version()}; seed 21; lat,lon,h,x,y,z,lat_r,lon_r,h_r (the last three from x,y,z by CartConvert -r)"]
    for (la, lo, h), x, r in zip(pts, fwd, inv):
        rows.append(",".join([repr(la), repr(lo), repr(h), *x, *r]))
    return rows


def enu(rnd, n=500):
    rows = [f"# GeographicLib CartConvert {version()}; seed 22; lat0,lon0,h0,lat,lon,h,east,north,up"]
    for _ in range(n):
        o = (rnd.uniform(-89, 89), rnd.uniform(-180, 180), rnd.uniform(-100, 5000))
        s = rnd.choice([0.01, 1, 10])
        t = (max(-90, min(90, o[0] + rnd.uniform(-s, s))), (o[1] + rnd.uniform(-s, s) + 180) % 360 - 180, rnd.uniform(-100, 4e5))
        (e, nn, u), = cart(["-l", repr(o[0]), repr(o[1]), repr(o[2])], [f"{t[0]!r} {t[1]!r} {t[2]!r}"])
        rows.append(",".join([*map(repr, o), *map(repr, t), e, nn, u]))
    return rows


def main():
    (OUT / "ecef_diff.csv").write_text("\n".join(ecef(random.Random(21))) + "\n")
    (OUT / "enu_diff.csv").write_text("\n".join(enu(random.Random(22))) + "\n")


if __name__ == "__main__":
    main()
