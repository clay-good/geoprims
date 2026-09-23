#!/usr/bin/env python3
"""More golden vectors for the three angle-parsing tools.

All three are exact arithmetic on angles, so the reference is exact arithmetic
on angles, done here a different way than the core does it:

- angle-arithmetic: each term is converted to an integer number of hundredths
  of an arcsecond with Python's `Fraction`, summed, normalized, and written
  back. Working in integers means the carry from seconds to minutes to degrees
  is exact, which is the whole point of the tool -- 59'59.995" rounds up to a
  whole degree, not to 60 minutes.
- bearing-difference: ((to - from + 180) mod 360) - 180, in exact rationals, so
  the answer at the +/-180 boundary is decided by the definition rather than by
  a float.
- format: the same rounding carried across components, and the ground
  resolution of the last digit from the WGS 84 radii of curvature.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_parse_more.py
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = "Exact rational arithmetic on arcseconds (tools/vectors/gen_parse_more.py)"
VER = "definition"
A = 6378137.0
INV_F = 298.257223563


def to_seconds(text):
    """An angle written D-M-S, D M S, or decimal degrees, in exact arcseconds."""
    text = text.strip()
    sign = -1 if text.startswith("-") else 1
    text = text.lstrip("+-")
    parts = [p for p in text.replace(":", "-").replace(" ", "-").split("-") if p]
    if len(parts) == 1:
        return sign * F(parts[0]) * 3600
    d = F(parts[0]) * 3600
    if len(parts) > 1:
        d += F(parts[1]) * 60
    if len(parts) > 2:
        d += F(parts[2])
    return sign * d


def dms(seconds, places=2):
    """D°MM'SS.ss" with the rounding carried into minutes and degrees."""
    neg = seconds < 0
    s = abs(seconds)
    unit = F(1, 10 ** places)
    # Round half away from zero on the last printed digit, then carry.
    ticks = int((s / unit) + F(1, 2)) if (s / unit) >= 0 else 0
    s = F(ticks) * unit
    deg, rem = divmod(s, 3600)
    minutes, sec = divmod(rem, 60)
    text = f"{int(deg)}°{int(minutes):02d}'{float(sec):0{3 + places}.{places}f}\""
    return ("-" if neg else "") + text


# (terms, normalize) -- terms are (angle text, subtract?)
ANGLE_CASES = [
    ([("10-00-00", False), ("20-00-00", False)], None),
    ([("0-00-01", False), ("0-00-01", False), ("0-00-01", False)], None),
    ([("89-59-59.99", False), ("0-00-00.01", False)], None),
    ([("1-00-00", False), ("0-59-59.5", True)], None),
    ([("100-30-00", False), ("200-45-30", False)], "0-360"),
    ([("359-00-00", False), ("2-00-00", False)], "0-360"),
    ([("10-00-00", False), ("350-00-00", False)], "0-360"),
    ([("45", False), ("45", False)], None),
    ([("45.5", False), ("0-30-00", True)], None),
    ([("-30-00-00", False), ("10-00-00", False)], None),
    ([("270-00-00", False), ("180-00-00", False)], "plus-minus-180"),
    ([("90-00-00", False), ("180-00-00", False)], "plus-minus-180"),
    ([("12-34-56.78", False), ("0-00-00.22", False)], None),
    ([("359-59-59.995", False), ("0-00-00.005", False)], "0-360"),
    ([("1-00-00", False), ("1-00-00", True), ("1-00-00", False)], None),
    ([("180-00-00", False), ("180-00-00", False)], "0-360"),
]

BEARING_CASES = [
    (0.0, 0.0), (0.0, 90.0), (0.0, 180.0), (0.0, 270.0),
    (90.0, 0.0), (180.0, 0.0), (270.0, 0.0),
    (359.0, 1.0), (1.0, 359.0), (45.0, 225.0), (225.0, 45.0),
    (0.5, 180.5), (359.5, 179.5), (100.0, 280.0), (280.0, 100.0),
    (123.456, 234.567), (10.0, 10.0),
]

# (lat, lon, style, decimals)
FORMAT_CASES = [
    (0.0, 0.0, "dms", 0), (0.0, 0.0, "dd", 6), (0.0, 0.0, "ddm", 3),
    (45.0, -75.0, "dms", 2), (45.0, -75.0, "ddm", 4), (45.0, -75.0, "dd", 8),
    (-33.8688, 151.2093, "dms", 1), (-33.8688, 151.2093, "ddm", 2),
    (51.4999999, -0.0000001, "dms", 0),
    (89.9999999, 179.9999999, "dms", 3),
    (-89.9999999, -179.9999999, "ddm", 3),
    (40.446111, -79.982222, "ddm", 3),
    (35.6762, 139.6503, "dd", 5),
    (-22.9068, -43.1729, "dms", 2),
    (60.0, 10.0, "ddm", 1),
    (1.3521, 103.8198, "dms", 4),
]


def meridian_m(lat):
    e2 = (2 - 1 / INV_F) / INV_F
    s = math.sin(math.radians(lat))
    return math.radians(A * (1 - e2) / (1 - e2 * s * s) ** 1.5)


def parallel_m(lat):
    e2 = (2 - 1 / INV_F) / INV_F
    s = math.sin(math.radians(lat))
    return math.radians(A / math.sqrt(1 - e2 * s * s)) * math.cos(math.radians(lat))


def angle_rows(start):
    rows = []
    for i, (terms, norm) in enumerate(ANGLE_CASES, start=start + 1):
        total = F(0)
        payload = []
        for text, sub in terms:
            total += -to_seconds(text) if sub else to_seconds(text)
            t = {"angle": text}
            if sub:
                t["operation"] = "subtract"
            payload.append(t)
        if norm == "0-360":
            total %= 3600 * 360
        elif norm == "plus-minus-180":
            total = (total + 3600 * 180) % (3600 * 360) - 3600 * 180
        inp = {"terms": payload}
        if norm:
            inp["normalize"] = norm
        rows.append((i, inp,
                     {"ok": True, "result.dms": dms(total),
                      "result.seconds.value": float(total),
                      "result.degrees.value": float(total / 3600)},
                     {"result.seconds.value": {"abs": 1e-9},
                      "result.degrees.value": {"abs": 1e-12},
                      "result.dms": {"abs": 0}}))
    return rows


def bearing_rows(start):
    rows = []
    for i, (a, b) in enumerate(BEARING_CASES, start=start + 1):
        d = float((F(b) - F(a) + 180) % 360 - 180)
        rows.append((i, {"from": a, "to": b},
                     {"ok": True, "result.difference.value": d},
                     {"result.difference.value": {"abs": 1e-12}}))
    return rows


def format_rows(start):
    rows = []
    for i, (lat, lon, style, decimals) in enumerate(FORMAT_CASES, start=start + 1):
        step = {"dd": 10.0 ** -decimals,
                "ddm": 10.0 ** -decimals / 60.0,
                "dms": 10.0 ** -decimals / 3600.0}[style]
        rows.append((i, {"lat": lat, "lon": lon, "style": style, "decimals": decimals},
                     {"ok": True,
                      "result.latitude_resolution.value": step * meridian_m(lat),
                      "result.longitude_resolution.value": step * parallel_m(lat)},
                     {"result.latitude_resolution.value": {"rel": 1e-9, "abs": 1e-9},
                      "result.longitude_resolution.value": {"rel": 1e-9, "abs": 1e-9}}))
    return rows


def main():
    plan = [
        ("geodesy.parse.angle-arithmetic", angle_rows),
        ("geodesy.parse.bearing-difference", bearing_rows),
        ("geodesy.parse.format", format_rows),
    ]
    for tool, build in plan:
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
