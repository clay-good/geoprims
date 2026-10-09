#!/usr/bin/env python3
"""Golden vectors for survey.gnss.dop, worked independently of the core.

A synthetic GPS constellation (six planes 60° apart at 55°, four satellites
each) is written out as a YUMA almanac, exactly as the Coast Guard publishes
one. Each satellite is then placed with the IS-GPS-200 almanac equations
(Table 20-IV), transcribed here from the standard with numpy: the Kepler
equation by fixed-point iteration (the core uses Newton's method), the site in
ECEF from the WGS 84 closed form, east-north-up by an explicit rotation, and
DOP from numpy's matrix inverse of GᵀG (the core uses Gauss-Jordan).

UTC to GPS time uses GPS - UTC = 18 s, in force since 2017-01-01.

Usage: python3 tools/vectors/gen_dop.py [--example]
"""
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

OUT = Path(__file__).resolve().parents[2] / "core/vectors/survey.gnss.dop.jsonl"
MU = 3.986005e14
OMEGA_E = 7.2921151467e-5
WEEK = 604800.0
GPS_EPOCH = datetime(1980, 1, 6, tzinfo=timezone.utc)
LEAP = 18.0
A_WGS, F_WGS = 6378137.0, 1 / 298.257223563

# The almanac's reference: full GPS week 2438 (ten-bit 390), toa 405504 s,
# which is 2026-10-02T16:38:06 GPS.
FULL_WEEK, TOA = 2438, 405504.0


def constellation(unhealthy=()):
    sats = []
    prn = 1
    for plane in range(6):
        for slot in range(4):
            sats.append({
                "prn": prn,
                "health": 63 if prn in unhealthy else 0,
                "e": 0.002 + 0.003 * ((prn * 7) % 5),
                "toa": TOA,
                "i0": math.radians(55.0 + 0.4 * ((prn * 3) % 5 - 2)),
                "omega_dot": -8.0e-9,
                "sqrt_a": 5153.6 + 0.05 * (prn % 7),
                "omega0": math.remainder(math.radians(60.0 * plane - 150.0), 2 * math.pi),
                "w": math.radians((37.0 * prn) % 360 - 180),
                "m0": math.remainder(math.radians(90.0 * slot + 15.0 * plane + 7.0 * (prn % 3)), 2 * math.pi),
                "week": FULL_WEEK % 1024,
            })
            prn += 1
    return sats


def yuma(sats):
    def e(x):
        # YUMA's mantissa form: 0.1166667938E-001.
        if x == 0:
            return "0.0000000000E+000"
        exp = math.floor(math.log10(abs(x))) + 1
        m = x / 10 ** exp
        return f"{m:.10f}E{exp:+04d}".replace("E+", "E+").replace("E-", "E-")

    out = []
    for s in sats:
        out.append(
            f"******** Week {s['week']} almanac for PRN-{s['prn']:02d} ********\n"
            f"ID:                         {s['prn']:02d}\n"
            f"Health:                     {s['health']:03d}\n"
            f"Eccentricity:               {e(s['e'])}\n"
            f"Time of Applicability(s):  {s['toa']:.4f}\n"
            f"Orbital Inclination(rad):   {s['i0']:.10f}\n"
            f"Rate of Right Ascen(r/s):  {e(s['omega_dot'])}\n"
            f"SQRT(A)  (m 1/2):           {s['sqrt_a']:.6f}\n"
            f"Right Ascen at Week(rad):  {e(s['omega0'])}\n"
            f"Argument of Perigee(rad):   {s['w']:.9f}\n"
            f"Mean Anom(rad):            {e(s['m0'])}\n"
            f"Af0(s):                     0.0000000000E+000\n"
            f"Af1(s/s):                   0.0000000000E+000\n"
            f"week:                        {s['week']}\n\n"
        )
    return "".join(out)


def sem(sats):
    """The same constellation as a SEM almanac: semicircles, and the
    inclination as an offset from 0.30 semicircles."""
    pi = math.pi
    out = [f"{len(sats)}  CURRENT.ALM", f" {sats[0]['week']} {int(sats[0]['toa'])}", ""]
    for s in sats:
        row = lambda *xs: " ".join(f"{x: .14E}" for x in xs)
        out += [str(s["prn"]), str(s["prn"] + 40), "0",
                row(s["e"], s["i0"] / pi - 0.30, s["omega_dot"] / pi),
                row(s["sqrt_a"], s["omega0"] / pi, s["w"] / pi),
                row(s["m0"] / pi, 0.0, 0.0),
                str(s["health"]), "11", ""]
    return "\n".join(out)


def reparse_sem(text):
    """The values as the core reads them back from a SEM text."""
    lines = [l for l in text.splitlines() if l.strip()]
    week, toa = (float(x) for x in lines[1].split())
    toks = " ".join(lines[2:]).split()
    out = []
    for k in range(0, len(toks), 14):
        v = [float(x) for x in toks[k:k + 14]]
        out.append({"prn": int(v[0]), "health": int(v[12]), "e": v[3], "toa": toa, "i0": (0.30 + v[4]) * math.pi,
                    "omega_dot": v[5] * math.pi, "sqrt_a": v[6], "omega0": v[7] * math.pi, "w": v[8] * math.pi,
                    "m0": v[9] * math.pi, "week": int(week)})
    return out


def reparse(text):
    """The values as the core will read them back from the printed text."""
    sats, cur = [], {}
    for line in text.splitlines():
        if line.startswith("*"):
            if cur:
                sats.append(cur)
            cur = {}
            continue
        if ":" not in line:
            continue
        k, v = line.split(":", 1)
        cur[k.strip().lower()] = float(v)
    if cur:
        sats.append(cur)
    return [{
        "prn": int(s["id"]), "health": int(s["health"]), "e": s["eccentricity"],
        "toa": s["time of applicability(s)"], "i0": s["orbital inclination(rad)"],
        "omega_dot": s["rate of right ascen(r/s)"], "sqrt_a": s["sqrt(a)  (m 1/2)"],
        "omega0": s["right ascen at week(rad)"], "w": s["argument of perigee(rad)"],
        "m0": s["mean anom(rad)"], "week": int(s["week"]),
    } for s in sats]


def sat_ecef(s, full_week, t):
    a = s["sqrt_a"] ** 2
    n = math.sqrt(MU / a**3)
    tk = t - (full_week * WEEK + s["toa"])
    m = s["m0"] + n * tk
    ea = m
    for _ in range(200):  # fixed point: E = M + e sin E
        ea = m + s["e"] * math.sin(ea)
    nu = math.atan2(math.sqrt(1 - s["e"] ** 2) * math.sin(ea), math.cos(ea) - s["e"])
    u = nu + s["w"]
    r = a * (1 - s["e"] * math.cos(ea))
    om = s["omega0"] + (s["omega_dot"] - OMEGA_E) * tk - OMEGA_E * s["toa"]
    p = np.array([r * math.cos(u), r * math.sin(u), 0.0])
    rz = lambda g: np.array([[math.cos(g), -math.sin(g), 0], [math.sin(g), math.cos(g), 0], [0, 0, 1]])
    rx = lambda g: np.array([[1, 0, 0], [0, math.cos(g), -math.sin(g)], [0, math.sin(g), math.cos(g)]])
    return rz(om) @ rx(s["i0"]) @ p


def site_ecef(lat, lon, h):
    e2 = F_WGS * (2 - F_WGS)
    p, l = math.radians(lat), math.radians(lon)
    n = A_WGS / math.sqrt(1 - e2 * math.sin(p) ** 2)
    return np.array([(n + h) * math.cos(p) * math.cos(l), (n + h) * math.cos(p) * math.sin(l), (n * (1 - e2) + h) * math.sin(p)])


def enu_matrix(lat, lon):
    p, l = math.radians(lat), math.radians(lon)
    return np.array([
        [-math.sin(l), math.cos(l), 0],
        [-math.sin(p) * math.cos(l), -math.sin(p) * math.sin(l), math.cos(p)],
        [math.cos(p) * math.cos(l), math.cos(p) * math.sin(l), math.sin(p)],
    ])


def horizon_at(h, az):
    if not h:
        return -math.inf
    if len(h) == 1:
        return h[0][1]
    h = sorted(h)
    # Unwrap around north: extend the list by one turn either side.
    ext = [(a - 360, e) for a, e in h] + h + [(a + 360, e) for a, e in h]
    for (a0, e0), (a1, e1) in zip(ext, ext[1:]):
        if a0 <= az < a1:
            return e0 + (e1 - e0) * (az - a0) / (a1 - a0)
    raise AssertionError


def solve(sats, lat, lon, h, start_utc, hours, step_min, mask, skyline):
    t0 = (start_utc - GPS_EPOCH).total_seconds() + LEAP
    start_week = math.floor(t0 / WEEK)
    site = site_ecef(lat, lon, h)
    rot = enu_matrix(lat, lon)
    healthy = [s for s in sats if s["health"] == 0]

    def full(w):
        return w + 1024 * round((start_week - w) / 1024) if w < 1024 else w

    def sky(t):
        out = []
        for s in healthy:
            v = rot @ (sat_ecef(s, full(s["week"]), t) - site)
            rng = np.linalg.norm(v)
            az = math.degrees(math.atan2(v[0], v[1])) % 360
            el = math.degrees(math.asin(v[2] / rng))
            out.append((s["prn"], az, el, v / rng))
        return out

    def used(az, el):
        return el >= mask and el >= horizon_at(skyline, az)

    def dop(los):
        if len(los) < 4:
            return None
        g = np.array([[-u[0], -u[1], -u[2], 1.0] for u in los])
        q = np.linalg.inv(g.T @ g)
        return [math.sqrt(np.trace(q)), math.sqrt(q[0, 0] + q[1, 1] + q[2, 2]), math.sqrt(q[0, 0] + q[1, 1]), math.sqrt(q[2, 2])]

    n = int(math.floor(hours * 60 / step_min)) + 1
    rows = []
    for k in range(n):
        t = t0 + k * step_min * 60
        los = [s[3] for s in sky(t) if used(s[1], s[2])]
        rows.append((t, len(los), dop(los)))
    ref = max(full(s["week"]) * WEEK + s["toa"] for s in sats)
    sats0 = sorted([s for s in sky(t0) if s[2] > 0], key=lambda s: -s[2])
    return rows, (t0 - ref) / 86400, sats0, used, len(healthy)


def case(sats, lat, lon, start, hours, step, mask=None, h=None, skyline=None, note="", fmt="yuma"):
    text = yuma(sats) if fmt == "yuma" else sem(sats)
    parsed = reparse(text) if fmt == "yuma" else reparse_sem(text)
    start_utc = datetime.fromisoformat(start.replace("Z", "+00:00"))
    rows, age, sky0, used, nh = solve(parsed, lat, lon, h or 0.0, start_utc, hours, step,
                                      10.0 if mask is None else mask, skyline or [])
    inp = {"almanac": text, "lat": lat, "lon": lon, "start": start, "duration": f"{hours} h", "step": f"{step} min"}
    if mask is not None:
        inp["mask"] = f"{mask} deg"
    if h is not None:
        inp["height"] = f"{h} m"
    if skyline:
        inp["horizon"] = [{"azimuth": f"{a} deg", "elevation": f"{e} deg"} for a, e in skyline]
    exp = {"ok": True, "result.visible": float(rows[0][1]), "result.fewest": float(min(r[1] for r in rows)),
           "result.healthy": float(nh), "result.almanac_age.value": age}
    d0 = rows[0][2]
    if d0:
        exp.update({"result.gdop": d0[0], "result.pdop": d0[1], "result.hdop": d0[2], "result.vdop": d0[3]})
    with_dop = [r for r in rows if r[2]]
    if with_dop:
        worst = max(with_dop, key=lambda r: r[2][1])
        exp["result.worst_pdop"] = worst[2][1]
    for i, r in enumerate(rows):
        exp[f"result.timeline.{i}.visible"] = float(r[1])
        if r[2]:
            exp[f"result.timeline.{i}.pdop"] = r[2][1]
    for i, s in enumerate(sky0[:3]):
        exp[f"result.satellites.{i}.prn"] = float(s[0])
        exp[f"result.satellites.{i}.azimuth.value"] = s[1]
        exp[f"result.satellites.{i}.elevation.value"] = s[2]
        exp[f"result.satellites.{i}.used"] = "yes" if used(s[1], s[2]) else "no"
    if age > 7:
        exp["meta.warnings.*.code"] = "ALMANAC_OLD"
    # Elevations to 1e-6°, DOPs to 1e-7: far below anything displayed, and
    # above what two independent Kepler solvers and rotation orders leave.
    tol = {}
    for k, v in exp.items():
        if isinstance(v, float):
            tol[k] = {"rel": 1e-7, "abs": 1e-6}
    return {"input": inp, "expect": exp, "source": f"IS-GPS-200 almanac equations worked independently in Python with numpy (tools/vectors/gen_dop.py): {note}",
            "sourceVersion": "IS-GPS-200N", "tolerance": tol}


def example_subset():
    """The four or more satellites above 10° from Denver at the example's start."""
    sats = constellation()
    text = yuma(sats)
    _, _, sky0, used, _ = solve(reparse(text), 39.74, -104.99, 0.0,
                                datetime(2026, 10, 3, 14, 0, tzinfo=timezone.utc), 2, 30, 10.0, [])
    keep = {s[0] for s in sky0 if used(s[1], s[2])}
    return [s for s in sats if s["prn"] in keep]


if __name__ == "__main__":
    full = constellation()
    if "--example" in sys.argv:
        print(json.dumps({"almanac": yuma(full), "lat": 39.74, "lon": -104.99, "start": "2026-10-03T14:00Z", "duration": "6 h", "step": "30 min"}, separators=(",", ":")))
        sys.exit()
    sub = example_subset()
    CASES = [
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30, note="the tool's example: the whole constellation over Denver for six hours"),
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 24, 60, note="the whole constellation over Denver for a day"),
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30, mask=15, note="a 15° mask"),
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30, mask=0, note="no mask"),
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30,
             skyline=[(0, 10), (90, 40), (180, 25), (270, 5)], note="a canyon skyline"),
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30,
             skyline=[(45, 60)], note="one flat skyline height all the way round"),
        case(full, -33.87, 151.21, "2026-10-04T00:00Z", 12, 60, note="Sydney"),
        case(full, 64.84, -147.72, "2026-10-04T06:00Z", 12, 60, h=200, note="Fairbanks, 200 m up"),
        case(full, 0.0, 0.0, "2026-10-02T18:00Z", 4, 20, note="the equator at the prime meridian"),
        case(full, 89.0, 30.0, "2026-10-03T00:00Z", 6, 60, note="near the North Pole, where no satellite climbs above about 45°"),
        case(full, 51.48, 0.0, "2026-10-03T09:30+01:00", 3, 15, note="a start given with an offset"),
        case(full, 39.74, -104.99, "2026-10-13T14:00Z", 4, 60, note="ten days after the almanac: ALMANAC_OLD"),
        case(full, 39.74, -104.99, "2026-09-30T14:00Z", 4, 60, note="a start before the almanac's reference time"),
        case(constellation(unhealthy=(3, 7, 11, 15, 19)), 39.74, -104.99, "2026-10-03T14:00Z", 6, 60,
             note="five satellites marked unhealthy and left out"),
        case(sub[:3], 39.74, -104.99, "2026-10-03T14:00Z", 1, 30, note="three satellites: too few for a DOP"),
        case(full, 35.0, 139.0, "2026-10-03T03:00Z", 48, 120, note="Tokyo over two days"),
        case(full, -54.8, -68.3, "2026-10-03T20:00Z", 8, 40, mask=5, note="Ushuaia, a 5° mask"),
        case(full, 19.5, -155.6, "2026-10-03T22:00Z", 2, 10, h=3400, note="Mauna Loa, 3,400 m up"),
        case(full, 40.0, -105.0, "2026-10-03T14:00Z", 0, 10, note="a window of one sample"),
        case(full, 47.6, -122.3, "2026-10-03T16:00Z", 10, 30, mask=20,
             skyline=[(200, 30), (340, 35)], note="Seattle, a 20° mask and trees to the south and north"),
        case(full, -1.3, 36.8, "2026-10-05T12:00Z", 6, 60, note="Nairobi, two days after the almanac"),
        case(full, 39.74, -104.99, "2026-10-03T07:00-07:00", 2, 30, note="the same instant as the first case, given in MST"),
        # Appended 2026-10-09: the same constellation as a SEM almanac.
        case(full, 39.74, -104.99, "2026-10-03T14:00Z", 6, 30, note="the example's constellation as a SEM almanac", fmt="sem"),
        case(full, -33.87, 151.21, "2026-10-04T00:00Z", 12, 60, note="Sydney, from a SEM almanac", fmt="sem"),
        case(constellation(unhealthy=(3, 7, 11, 15, 19)), 39.74, -104.99, "2026-10-03T14:00Z", 6, 60,
             note="a SEM almanac with five satellites marked unhealthy", fmt="sem"),
        case(full, 39.74, -104.99, "2026-10-13T14:00Z", 4, 60, note="a SEM almanac ten days old: ALMANAC_OLD", fmt="sem"),
    ]
    with OUT.open("w") as f:
        for i, row in enumerate(CASES, 1):
            f.write(json.dumps({"id": f"v{i:03d}", **row}) + "\n")
    print(f"wrote {len(CASES)} vectors to {OUT}")
