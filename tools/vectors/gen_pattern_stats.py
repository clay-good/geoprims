#!/usr/bin/env python3
"""Golden vectors for drone.mission.pattern-stats, with every leg solved by
GeographicLib's GeodSolve CLI, which is independent of the Rust port the core
uses. Needs GeodSolve on the PATH."""
import json
import random
import subprocess
import sys
from pathlib import Path

SRC = "GeographicLib GeodSolve -i on each leg, summed in Python (tools/vectors/gen_pattern_stats.py)"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SPEC = "add-drone-suite scenarios"
TURN = 5.0


def inverse(pairs):
    """[(azi1, azi2, s12)] for [(lat1, lon1, lat2, lon2)], in fixed-point so no exponent is read as a hemisphere."""
    text = "".join(f"{a:.12f} {b:.12f} {c:.12f} {d:.12f}\n" for a, b, c, d in pairs)
    out = subprocess.run(["GeodSolve", "-i", "-p", "9"], input=text, capture_output=True, text=True, check=True).stdout
    return [tuple(float(x) for x in line.split()) for line in out.strip().splitlines()]


def stats(points, speed=None):
    legs = inverse([(*points[i], *points[i + 1]) for i in range(len(points) - 1)])
    from_start = inverse([(*points[0], *p) for p in points[1:]])
    changes = [abs((legs[i][0] - legs[i - 1][1] + 540) % 360 - 180) for i in range(1, len(legs))]
    if any(abs(c - TURN) < 0.01 for c in changes):
        return None  # too close to the threshold to pin a count
    total = sum(s for _, _, s in legs)
    exp = {"result.path_length.value": total / 1000, "result.legs": float(len(legs)), "result.turns": float(sum(c > TURN for c in changes)),
           "result.longest_leg.value": max(s for _, _, s in legs), "result.farthest_from_start.value": max(s for _, _, s in from_start),
           "result.total_turning.value": float(sum(changes))}
    if speed:
        exp["result.flight_time.value"] = total / speed / 60
    return exp


def vec(i, points, speed=None, src=SRC, ver=VER):
    exp = stats(points, speed)
    if exp is None:
        return None
    exp["ok"] = True
    inp = {"waypoints": [{"lat": la, "lon": lo} for la, lo in points]}
    if speed:
        inp["groundspeed"] = f"{speed} m/s"
    # Lengths to a micrometer (CI's x86 and a Mac's ARM differ near 1e-8 m in summed geodesics); counts exact.
    tol = {k: ({"abs": 0} if k in ("result.legs", "result.turns") else {"rel": 1e-9, "abs": 1e-6}) for k, v in exp.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": ver, "tolerance": tol}


def refusal(i, inp, field):
    return {"id": f"v{i:03d}", "input": inp, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": field},
            "source": SPEC, "sourceVersion": "2026", "tolerance": {}}


def main():
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    rng = random.Random(55401)
    out = [vec(1, [(44.98, -93.27), (44.9818, -93.27), (44.9818, -93.2662), (44.98, -93.2662), (44.98, -93.27)], 8)]
    # One leg, a straight three-point line, and a there-and-back.
    out.append(vec(2, [(40.0, -105.0), (40.001, -105.0)]))
    out.append(vec(3, [(0.0, 10.0), (0.0, 10.001), (0.0, 10.002)], 5))
    out.append(vec(4, [(51.5, -0.12), (51.501, -0.118), (51.5, -0.12)], 12))
    homes = [(40.0, -105.0), (-33.9, 151.2), (64.1, -21.9), (1.3, 103.8), (-54.8, -68.3), (78.2, 15.6), (35.7, 139.7), (-1.3, 36.8)]
    while len(out) < 20:
        lat, lon = homes[len(out) % len(homes)]
        points = [(lat, lon)]
        for _ in range(rng.randrange(2, 13)):
            points.append((round(points[-1][0] + rng.uniform(-0.004, 0.004), 6), round(points[-1][1] + rng.uniform(-0.006, 0.006), 6)))
        v = vec(len(out) + 1, points, rng.choice([None, 5, 8, 10, 15]))
        if v:
            out.append(v)
    two = [{"lat": 40.0, "lon": -105.0}, {"lat": 40.001, "lon": -105.0}]
    out.append(refusal(len(out) + 1, {"waypoints": [two[0], two[0], two[1]]}, "/waypoints/1"))
    out.append(refusal(len(out) + 1, {"waypoints": two, "groundspeed": "0 m/s"}, "/groundspeed"))
    out.append(refusal(len(out) + 1, {"waypoints": [{"lat": 91, "lon": 0}, two[1]]}, "/waypoints/0/lat"))
    (out_dir / "drone.mission.pattern-stats.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
