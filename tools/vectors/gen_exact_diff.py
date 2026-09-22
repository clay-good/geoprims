#!/usr/bin/env python3
"""Differential data for the exact geodesic method (gp_geo::exact), from
GeographicLib's GeodSolve -E (GeodesicExact, elliptic integrals), on oblate
ellipsoids with f from 1/40 to 1/2 and a = 6,378,137 m.

Writes core/crates/gp-navigation/tests/data/exact_diff.txt, one geodesic per
line: "kind f lat1 lon1 azi1 lat2 lon2 azi2 s12 a12 m12 M12 M21 S12", the
columns of GeodSolve -f after I (from the inverse problem, so the shortest
geodesic) or D (from the direct, which may run past it). Half are each,
seeded, with nearly antipodal pairs left out (the tool refuses those). Numbers
go in fixed-point because GeographicLib reads an exponent's "e" as a hemisphere.
"""
import math
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/crates/gp-navigation/tests/data/exact_diff.txt"
A = 6378137.0
FS = [1 / 40, 1 / 20, 1 / 10, 1 / 5, 1 / 3, 1 / 2]


def solve(f, lines, inverse):
    cmd = ["GeodSolve", "-E", "-f", "-p", "12", "-e", f"{A}", f"{f:.17f}"] + (["-i"] if inverse else [])
    out = subprocess.run(cmd, input="".join(l + "\n" for l in lines), capture_output=True, text=True, check=True).stdout
    return [l.split() for l in out.splitlines() if l.strip()]


def main():
    rnd = random.Random(20260922)
    rows = []
    for f in FS:
        inv, direct = [], []
        while len(inv) < 40:
            la1, la2 = math.degrees(math.asin(rnd.uniform(-1, 1))), math.degrees(math.asin(rnd.uniform(-1, 1)))
            lo1, lo2 = rnd.uniform(-180, 180), rnd.uniform(-180, 180)
            p1, p2, dl = math.radians(la1), math.radians(la2), math.radians(lo2 - lo1)
            if math.degrees(math.acos(max(-1, min(1, math.sin(p1) * math.sin(p2) + math.cos(p1) * math.cos(p2) * math.cos(dl))))) > 150:
                continue
            inv.append(f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}")
        for _ in range(40):
            direct.append(f"{math.degrees(math.asin(rnd.uniform(-1, 1))):.12f} {rnd.uniform(-180, 180):.12f} {rnd.uniform(0, 360):.12f} {rnd.uniform(1e3, 1.5e7):.6f}")
        rows += [f"I {f!r} " + " ".join(r) for r in solve(f, inv, True)]
        rows += [f"D {f!r} " + " ".join(r) for r in solve(f, direct, False)]
    OUT.write_text("\n".join(rows) + "\n")
    print(f"{len(rows)} geodesics")


if __name__ == "__main__":
    main()
