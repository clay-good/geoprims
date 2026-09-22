#!/usr/bin/env python3
"""Golden vectors for geometry.distance.tracks, with distances from
GeographicLib's GeodSolve (C++, independent of the tool's geographiclib-rs).
Parallel tracks along the equator are a meridian arc apart by every measure;
resampling one track changes the Fréchet distance to the diagonal to a
neighbor but leaves the Hausdorff distance alone; a single displaced point
sets the Fréchet distance at its own index. Requires GeodSolve."""
import json
import subprocess
import sys
from pathlib import Path

SRC = "Distances by GeographicLib's GeodSolve (C++), couplings worked by hand (tools/vectors/gen_distance.py)"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
A, F = 6378137.0, 1 / 298.257223563


def inv(p, q):
    o = subprocess.run(["GeodSolve", "-i", "-p", "12"], input=f"{p[0]} {p[1]} {q[0]} {q[1]}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(o[2])


def direct(p, az, s):
    o = subprocess.run(["GeodSolve", "-p", "12"], input=f"{p[0]} {p[1]} {az} {s}\n", capture_output=True, text=True, check=True).stdout.split()
    return float(o[0]), float(o[1])


def T(pts):
    return [{"lat": a, "lon": b} for a, b in pts]


def vec(i, inp, exp, src=SRC, ver=VER, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol if k.endswith(".value") else 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def main():
    out = []
    dlat = 25 / (A * (1 - F * (2 - F))) * 57.29577951308232
    a = [(0.0, 0.001 * k) for k in range(11)]
    b = [(dlat, 0.001 * k) for k in range(11)]
    gap = inv(a[0], b[0])
    out.append(vec(len(out) + 1, {"track_a": T(a), "track_b": T(b)}, {"result.frechet.value": gap, "result.hausdorff.value": gap, "result.closest.value": gap}))
    # Every second point of the second track: the odd points of the first couple diagonally.
    b2 = b[::2]
    out.append(vec(len(out) + 1, {"track_a": T(a), "track_b": T(b2)}, {"result.frechet.value": inv(a[1], b2[0]), "result.hausdorff.value": gap, "result.closest.value": gap}))
    # The same track with one point pushed 30 m north: the Fréchet distance is that push, at that point.
    base = [(40.0, -105.0 + 0.001 * k) for k in range(10)]
    moved = list(base)
    moved[5] = direct(base[5], 0.0, 30.0)
    out.append(vec(len(out) + 1, {"track_a": T(base), "track_b": T(moved)},
                   {"result.frechet.value": 30.0, "result.frechet_a": 6, "result.frechet_b": 6, "result.hausdorff.value": 30.0, "result.closest.value": 0.0}, SPEC, tol=1e-3))
    # Tracks that cross have no gap between them.
    x1 = [(40.0, -105.0), (40.002, -104.998)]
    x2 = [(40.0, -104.998), (40.002, -105.0)]
    out.append(vec(len(out) + 1, {"track_a": T(x1), "track_b": T(x2)}, {"result.closest.value": 0.0, "result.frechet.value": inv(x1[0], x2[0])}))
    # Reversed direction: the Fréchet distance sees the order, the Hausdorff distance does not.
    rev = b[::-1]
    out.append(vec(len(out) + 1, {"track_a": T(a), "track_b": T(rev)}, {"result.hausdorff.value": gap, "result.frechet.value": inv(a[0], rev[0])}))
    big = [(0.0, 0.0001 * k) for k in range(501)]
    out.append(vec(len(out) + 1, {"track_a": T(big), "track_b": T(big)}, {"ok": False, "error.code": "LIMIT_EXCEEDED"}, SPEC, "2026"))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.distance.tracks.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
