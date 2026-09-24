#!/usr/bin/env python3
"""The three other readings of the wind triangle (promotion of find-wind,
course-from-heading, and tas-from-groundspeed).

Each is checked two ways. The FAA's worked example (PHAK, FAA-H-8083-25C,
chapter 16, figures 16-19 to 16-22: true course 090°, TAS 120 kt, wind 045°
at 40 kt, drawn to scale as heading 076° and groundspeed 88 kt) is read the
three other ways, to the drawing's 1° and 1 kt. And an independent vector
solution in Python gives the rest, at random headings, airspeeds, and winds.

Appends to each tool's vector file once, after its existing lines, which stay
byte for byte."""
import json
import math
import random
from pathlib import Path

PHAK = "FAA Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), chapter 16, figures 16-19 to 16-22"
PHAK_VER = "FAA-H-8083-25C (2023)"
SRC = "Wind-triangle vectors solved independently in Python (tools/vectors/gen_wind_inverse.py)"
VER = "2026-09-24"


def toward(deg, speed):
    a = math.radians(deg)
    return speed * math.sin(a), speed * math.cos(a)


def bearing(x, y):
    return math.degrees(math.atan2(x, y)) % 360


def signed(d):
    return (d + 180) % 360 - 180


def near_north(*angles):
    return any(min(a % 360, 360 - a % 360) < 0.05 for a in angles)


def vec(n, inp, exp, src=SRC, ver=VER, tol=1e-9):
    e = dict(exp)
    e["ok"] = True
    return {"id": f"v{n:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver,
            "tolerance": {k: {"rel": 0, "abs": tol} for k in exp}}


def find_wind(start, rng):
    rows = [vec(start, {"heading": 76, "tas": 120, "track": 90, "groundspeed": 88},
                {"result.wind_direction.value": 45.0, "result.wind_speed.value": 40.0}, PHAK, PHAK_VER, tol=1.0)]
    while len(rows) < 17:
        h, tas = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(60, 250), 1)
        wd, ws = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(5, 60), 1)
        ax, ay = toward(h, tas)
        wx, wy = toward(wd + 180, ws)
        tr, gs = bearing(ax + wx, ay + wy), math.hypot(ax + wx, ay + wy)
        tr, gs = round(tr, 2), round(gs, 2)
        gx, gy = toward(tr, gs)
        fx, fy = gx - ax, gy - ay
        d, s = (bearing(-fx, -fy), math.hypot(fx, fy))
        if near_north(d, h, tr):
            continue
        rows.append(vec(start + len(rows), {"heading": h, "tas": tas, "track": tr, "groundspeed": gs},
                        {"result.wind_direction.value": d, "result.wind_speed.value": s}))
    return rows


def course_from_heading(start, rng):
    rows = [vec(start, {"heading": "76 deg", "tas": "120 kt", "wind_direction": "45 deg", "wind_speed": "40 kt"},
                {"result.course.value": 90.0, "result.groundspeed.value": 88.0}, PHAK, PHAK_VER, tol=1.0)]
    while len(rows) < 14:
        h, tas = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(60, 250), 1)
        wd, ws = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(0, 60), 1)
        ax, ay = toward(h, tas)
        wx, wy = toward(wd + 180, ws)
        c, gs = bearing(ax + wx, ay + wy), math.hypot(ax + wx, ay + wy)
        if near_north(c, h):
            continue
        rows.append(vec(start + len(rows), {"heading": f"{h} deg", "tas": f"{tas} kt", "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"},
                        {"result.course.value": c, "result.groundspeed.value": gs, "result.drift_angle.value": signed(c - h)}))
    return rows


def tas_from_groundspeed(start, rng):
    rows = [vec(start, {"course": "90 deg", "groundspeed": "88 kt", "wind_direction": "45 deg", "wind_speed": "40 kt"},
                {"result.tas.value": 120.0, "result.heading.value": 76.0}, PHAK, PHAK_VER, tol=1.0)]
    while len(rows) < 14:
        c, gs = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(60, 250), 1)
        wd, ws = round(rng.uniform(0, 360), 1) % 360, round(rng.uniform(0, 60), 1)
        gx, gy = toward(c, gs)
        wx, wy = toward(wd + 180, ws)
        h, tas = bearing(gx - wx, gy - wy), math.hypot(gx - wx, gy - wy)
        if near_north(c, h):
            continue
        rows.append(vec(start + len(rows), {"course": f"{c} deg", "groundspeed": f"{gs} kt", "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"},
                        {"result.tas.value": tas, "result.heading.value": h, "result.wind_correction_angle.value": signed(h - c)}))
    return rows


def main():
    rng = random.Random(1620)
    for tool, make in [("find-wind", find_wind), ("course-from-heading", course_from_heading),
                       ("tas-from-groundspeed", tas_from_groundspeed)]:
        path = Path(f"core/vectors/aviation.wind.{tool}.jsonl")
        lines = path.read_text().splitlines()
        if any("gen_wind_inverse" in line or "PHAK" in line and "16-19" in line for line in lines):
            print(f"{path.name}: already appended")
            continue
        rows = make(len(lines) + 1, rng)
        lines += [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]
        path.write_text("\n".join(lines) + "\n")
        print(f"{path.name}: {len(lines)}")


if __name__ == "__main__":
    main()
