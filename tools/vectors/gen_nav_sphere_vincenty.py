#!/usr/bin/env python3
"""Golden vectors for the spherical and Vincenty geodesic tools.

These four cannot be checked against GeographicLib, because they are not
Karney's algorithm -- that is the point of them. A spherical answer and a
Vincenty answer are both *deliberately* different from the exact ellipsoidal
one, and a tool that quietly returned Karney's numbers would be useless for the
job these do: checking legacy software and reproducing published figures.

So the reference is an implementation of each algorithm from its own published
formulas:

- spherical: the great-circle direct and inverse formulas on a sphere of
  6,371.008771 km, the mean radius of WGS 84. The inverse uses the atan2 form
  rather than the plain arccos, which loses precision on short lines.
- Vincenty: the 1975 paper's iteration, written out here in full -- the
  reduced latitudes, the lambda iteration to 1e-12 radians, the A and B series
  in u squared, and the sigma correction. It is not Karney's method and does
  not converge for nearly antipodal points, which is exactly the behaviour the
  tool documents.

The tools themselves report the difference from Karney alongside their answer,
and those differences are what the vectors also pin.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_nav_sphere_vincenty.py
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SPH_SRC = ("The great-circle formulas on a sphere of 6,371.008771 km, transcribed from their "
           "definitions (tools/vectors/gen_nav_sphere_vincenty.py)")
SPH_VER = "definition"
VIN_SRC = ("Vincenty's 1975 inverse and direct methods, implemented from the paper's own formulas "
           "(tools/vectors/gen_nav_sphere_vincenty.py)")
VIN_VER = "Vincenty (1975), Survey Review XXIII no. 176"

R_KM = 6371.008771
A = 6378137.0
F = 1 / 298.257223563
B = A * (1 - F)


def sph_inverse(la1, lo1, la2, lo2):
    """(distance km, initial course, final course) on the sphere."""
    p1, p2 = math.radians(la1), math.radians(la2)
    dl = math.radians(lo2 - lo1)
    sig = math.atan2(
        math.hypot(math.cos(p2) * math.sin(dl),
                   math.cos(p1) * math.sin(p2) - math.sin(p1) * math.cos(p2) * math.cos(dl)),
        math.sin(p1) * math.sin(p2) + math.cos(p1) * math.cos(p2) * math.cos(dl),
    )
    c1 = math.degrees(math.atan2(
        math.sin(dl) * math.cos(p2),
        math.cos(p1) * math.sin(p2) - math.sin(p1) * math.cos(p2) * math.cos(dl))) % 360.0
    back = math.degrees(math.atan2(
        math.sin(-dl) * math.cos(p1),
        math.cos(p2) * math.sin(p1) - math.sin(p2) * math.cos(p1) * math.cos(dl)))
    c2 = (back + 180.0) % 360.0
    return R_KM * sig, c1, c2


def sph_direct(la1, lo1, course, dist_km):
    """(lat2, lon2, final course) on the sphere."""
    p1, l1, th = math.radians(la1), math.radians(lo1), math.radians(course)
    d = dist_km / R_KM
    p2 = math.asin(math.sin(p1) * math.cos(d) + math.cos(p1) * math.sin(d) * math.cos(th))
    l2 = l1 + math.atan2(math.sin(th) * math.sin(d) * math.cos(p1),
                         math.cos(d) - math.sin(p1) * math.sin(p2))
    lat2 = math.degrees(p2)
    lon2 = (math.degrees(l2) + 540.0) % 360.0 - 180.0
    _, back, _ = sph_inverse(lat2, lon2, la1, lo1)
    return lat2, lon2, (back + 180.0) % 360.0


def vincenty_inverse(la1, lo1, la2, lo2):
    """Vincenty 1975: (distance m, azimuth1, azimuth2), or None if it will not
    converge -- which happens for nearly antipodal points and is the known
    limitation the tool exists to demonstrate."""
    L = math.radians(lo2 - lo1)
    U1 = math.atan((1 - F) * math.tan(math.radians(la1)))
    U2 = math.atan((1 - F) * math.tan(math.radians(la2)))
    sU1, cU1, sU2, cU2 = math.sin(U1), math.cos(U1), math.sin(U2), math.cos(U2)
    lam = L
    for _ in range(200):
        sL, cL = math.sin(lam), math.cos(lam)
        sin_sig = math.hypot(cU2 * sL, cU1 * sU2 - sU1 * cU2 * cL)
        if sin_sig == 0:
            return 0.0, 0.0, 0.0  # coincident
        cos_sig = sU1 * sU2 + cU1 * cU2 * cL
        sig = math.atan2(sin_sig, cos_sig)
        sin_alpha = cU1 * cU2 * sL / sin_sig
        cos2_alpha = 1 - sin_alpha * sin_alpha
        cos_2sm = cos_sig - 2 * sU1 * sU2 / cos2_alpha if cos2_alpha != 0 else 0.0
        C = F / 16 * cos2_alpha * (4 + F * (4 - 3 * cos2_alpha))
        prev = lam
        lam = L + (1 - C) * F * sin_alpha * (
            sig + C * sin_sig * (cos_2sm + C * cos_sig * (-1 + 2 * cos_2sm * cos_2sm))
        )
        if abs(lam - prev) < 1e-12:
            break
    else:
        return None
    u2 = cos2_alpha * (A * A - B * B) / (B * B)
    Acoef = 1 + u2 / 16384 * (4096 + u2 * (-768 + u2 * (320 - 175 * u2)))
    Bcoef = u2 / 1024 * (256 + u2 * (-128 + u2 * (74 - 47 * u2)))
    d_sig = Bcoef * sin_sig * (
        cos_2sm + Bcoef / 4 * (
            cos_sig * (-1 + 2 * cos_2sm ** 2)
            - Bcoef / 6 * cos_2sm * (-3 + 4 * sin_sig ** 2) * (-3 + 4 * cos_2sm ** 2)
        )
    )
    s = B * Acoef * (sig - d_sig)
    sL, cL = math.sin(lam), math.cos(lam)
    a1 = math.degrees(math.atan2(cU2 * sL, cU1 * sU2 - sU1 * cU2 * cL)) % 360.0
    a2 = math.degrees(math.atan2(cU1 * sL, -sU1 * cU2 + cU1 * sU2 * cL)) % 360.0
    return s, a1, a2


def vincenty_direct(la1, lo1, azi, s):
    """Vincenty 1975 direct: (lat2, lon2, azimuth2)."""
    a1 = math.radians(azi)
    U1 = math.atan((1 - F) * math.tan(math.radians(la1)))
    sU1, cU1 = math.sin(U1), math.cos(U1)
    sig1 = math.atan2(math.tan(U1), math.cos(a1))
    sin_alpha = cU1 * math.sin(a1)
    cos2_alpha = 1 - sin_alpha * sin_alpha
    u2 = cos2_alpha * (A * A - B * B) / (B * B)
    Acoef = 1 + u2 / 16384 * (4096 + u2 * (-768 + u2 * (320 - 175 * u2)))
    Bcoef = u2 / 1024 * (256 + u2 * (-128 + u2 * (74 - 47 * u2)))
    sig = s / (B * Acoef)
    for _ in range(200):
        cos_2sm = math.cos(2 * sig1 + sig)
        d_sig = Bcoef * math.sin(sig) * (
            cos_2sm + Bcoef / 4 * (
                math.cos(sig) * (-1 + 2 * cos_2sm ** 2)
                - Bcoef / 6 * cos_2sm * (-3 + 4 * math.sin(sig) ** 2) * (-3 + 4 * cos_2sm ** 2)
            )
        )
        prev = sig
        sig = s / (B * Acoef) + d_sig
        if abs(sig - prev) < 1e-12:
            break
    cos_2sm = math.cos(2 * sig1 + sig)
    ss, cs = math.sin(sig), math.cos(sig)
    lat2 = math.atan2(
        sU1 * cs + cU1 * ss * math.cos(a1),
        (1 - F) * math.hypot(sin_alpha, sU1 * ss - cU1 * cs * math.cos(a1)),
    )
    lam = math.atan2(ss * math.sin(a1), cU1 * cs - sU1 * ss * math.cos(a1))
    C = F / 16 * cos2_alpha * (4 + F * (4 - 3 * cos2_alpha))
    L = lam - (1 - C) * F * sin_alpha * (
        sig + C * ss * (cos_2sm + C * cs * (-1 + 2 * cos_2sm ** 2))
    )
    lon2 = (math.degrees(math.radians(lo1) + L) + 540.0) % 360.0 - 180.0
    a2 = math.degrees(math.atan2(sin_alpha, -sU1 * ss + cU1 * cs * math.cos(a1))) % 360.0
    return math.degrees(lat2), lon2, a2


PAIRS = [
    (40.6413, -73.7781, 51.47, -0.4543), (-33.9, 151.2, 1.35, 103.99),
    (0.0, 0.0, 0.0, 90.0), (0.0, 0.0, 10.0, 0.0), (45.0, 0.0, 45.0, 90.0),
    (51.5074, -0.1278, 35.6762, 139.6503), (-22.9068, -43.1729, -33.8688, 151.2093),
    (60.0, 5.0, 60.5, 7.0), (1.3521, 103.8198, 1.2897, 103.8501),
    (-45.0, 170.0, -44.0, 171.0), (19.4326, -99.1332, 40.4168, -3.7038),
    (64.1466, -21.9426, 78.2232, 15.6267), (35.0, -120.0, 35.0, -75.0),
    (-10.0, 20.0, 10.0, 20.0), (25.2048, 55.2708, 1.3521, 103.8198),
    (-54.8019, -68.3030, -34.6037, -58.3816),
]
# (lat, lon, course, distance km)
STARTS = [
    (40.6413, -73.7781, 51.0, 5540.0), (0.0, 0.0, 90.0, 10000.0),
    (0.0, 0.0, 0.0, 1000.0), (45.0, -75.0, 225.0, 500.0),
    (-33.8688, 151.2093, 315.0, 2000.0), (51.5074, -0.1278, 45.0, 100.0),
    (60.0, 5.0, 180.0, 3000.0), (-45.0, 170.0, 90.0, 7500.0),
    (89.0, 0.0, 180.0, 500.0), (25.2048, 55.2708, 270.0, 4000.0),
    (1.3521, 103.8198, 30.0, 50.0), (-22.9068, -43.1729, 135.0, 1500.0),
    (35.6762, 139.6503, 60.0, 250.0), (19.4326, -99.1332, 0.0, 6000.0),
    (78.2232, 15.6267, 90.0, 800.0), (-60.0, -60.0, 45.0, 2500.0),
]


def sph_inverse_rows(start):
    rows = []
    for i, (la1, lo1, la2, lo2) in enumerate(PAIRS, start=start + 1):
        d, c1, c2 = sph_inverse(la1, lo1, la2, lo2)
        expect = {"ok": True, "result.distance.value": d}
        tol = {"result.distance.value": {"abs": 1e-9}}
        # A course is ill-determined along a meridian near the pole and on a
        # line the two points share exactly; those cases pin the distance only.
        if abs(la1) < 89.0 and (la1, lo1) != (la2, lo2):
            expect["result.initial_course.value"] = c1
            expect["result.final_course.value"] = c2
            tol["result.initial_course.value"] = {"abs": 1e-9}
            tol["result.final_course.value"] = {"abs": 1e-9}
        rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2},
                     expect, tol, SPH_SRC, SPH_VER))
    return rows


def sph_direct_rows(start):
    rows = []
    for i, (la, lo, course, dist) in enumerate(STARTS, start=start + 1):
        lat2, lon2, c2 = sph_direct(la, lo, course, dist)
        rows.append((i, {"lat1": la, "lon1": lo, "course": f"{course:g} deg",
                         "distance": f"{dist:g} km"},
                     {"ok": True, "result.lat2.value": lat2, "result.lon2.value": lon2,
                      "result.final_course.value": c2},
                     {"result.lat2.value": {"abs": 1e-9},
                      "result.lon2.value": {"abs": 1e-9},
                      "result.final_course.value": {"abs": 1e-8}},
                     SPH_SRC, SPH_VER))
    return rows


def vin_inverse_rows(start):
    rows = []
    i = start
    for la1, lo1, la2, lo2 in PAIRS:
        v = vincenty_inverse(la1, lo1, la2, lo2)
        if v is None:
            continue
        s, a1, a2 = v
        i += 1
        expect = {"ok": True, "result.distance.value": s / 1000.0}
        tol = {"result.distance.value": {"abs": 1e-8}}
        if abs(la1) < 89.0:
            expect["result.azimuth1.value"] = a1
            expect["result.azimuth2.value"] = a2
            tol["result.azimuth1.value"] = {"abs": 1e-9}
            tol["result.azimuth2.value"] = {"abs": 1e-9}
        rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2},
                     expect, tol, VIN_SRC, VIN_VER))
    return rows


def vin_direct_rows(start):
    rows = []
    for i, (la, lo, azi, dist) in enumerate(STARTS, start=start + 1):
        lat2, lon2, a2 = vincenty_direct(la, lo, azi, dist * 1000.0)
        rows.append((i, {"lat1": la, "lon1": lo, "azimuth": f"{azi:g} deg",
                         "distance": f"{dist:g} km"},
                     {"ok": True, "result.lat2.value": lat2, "result.lon2.value": lon2,
                      "result.azimuth2.value": a2},
                     {"result.lat2.value": {"abs": 1e-9},
                      "result.lon2.value": {"abs": 1e-9},
                      "result.azimuth2.value": {"abs": 1e-8}},
                     VIN_SRC, VIN_VER))
    return rows


def main():
    plan = [
        ("navigation.geodesic.spherical-inverse", sph_inverse_rows),
        ("navigation.geodesic.spherical-direct", sph_direct_rows),
        ("navigation.geodesic.vincenty-inverse", vin_inverse_rows),
        ("navigation.geodesic.vincenty-direct", vin_direct_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 10:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol, src, ver in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": src, "sourceVersion": ver, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
