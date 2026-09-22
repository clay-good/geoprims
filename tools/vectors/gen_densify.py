#!/usr/bin/env python3
"""Golden vectors for geometry.shape.densify, from GeographicLib's GeodSolve on
WGS 84: each edge's length and azimuth by the inverse problem, n equal pieces
of at most the longest length, and the new vertices by the direct problem.
Numbers go in fixed-point because GeographicLib reads an exponent's "e" as a
hemisphere."""
import json
import math
import subprocess
import sys
from pathlib import Path

SRC, VER = "GeographicLib GeodSolve inverse and direct on WGS 84 (tools/vectors/gen_densify.py)", "GeographicLib 2.x"


def geod(args, line):
    out = subprocess.run(["GeodSolve", "-p", "12"] + args, input=line + "\n", capture_output=True, text=True, check=True).stdout
    return [float(v) for v in out.split()]


def densify(pts, max_m, ring):
    edges = len(pts) if ring else len(pts) - 1
    out, total, longest = [], 0.0, 0.0
    for i in range(edges):
        a, b = pts[i], pts[(i + 1) % len(pts)]
        az, _, s = geod(["-i"], f"{a[0]:.12f} {a[1]:.12f} {b[0]:.12f} {b[1]:.12f}")
        n = max(1, math.ceil(s / max_m))
        out.append(a)
        for k in range(1, n):
            la, lo, _ = geod([], f"{a[0]:.12f} {a[1]:.12f} {az:.15f} {s * k / n:.9f}")
            out.append((la, lo))
        total += s
        longest = max(longest, s / n)
    if not ring:
        out.append(pts[-1])
    return out, total, longest


CASES = [
    ([(39.8561, -104.6737), (41.9742, -87.9073)], 200e3, False),
    ([(40.6413, -73.7781), (51.47, -0.4543)], 500e3, False),
    ([(0, 0), (0, 10), (10, 10), (10, 0)], 300e3, True),
    ([(-33.9, 151.2), (-37.8, 145.0), (-31.95, 115.86)], 250e3, False),
    ([(64.1, -21.9), (61.0, -150.0)], 1000e3, False),
    ([(10, 170), (12, -170)], 100e3, False),
]


def main():
    out = []
    for i, (pts, mx, ring) in enumerate(CASES, 1):
        res, total, longest = densify(pts, mx, ring)
        inp = {"points": [{"lat": a, "lon": b} for a, b in pts], "max_length": f"{mx / 1000:g} km"}
        if ring:
            inp["shape"] = "polygon"
        exp = {"result.vertices_out": float(len(res)), "result.length.value": total / 1000, "result.longest_piece.value": longest / 1000}
        for k in (1, len(res) // 2):
            exp[f"result.densified.{k}.lat.value"] = float(res[k][0])
            exp[f"result.densified.{k}.lon.value"] = float(((res[k][1] + 180) % 360) - 180)
        exp["ok"] = True
        tol = {k: ({"abs": 1e-9} if "densified" in k else {"rel": 1e-12, "abs": 1e-9}) for k, v in exp.items() if isinstance(v, float)}
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.shape.densify.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
