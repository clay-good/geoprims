#!/usr/bin/env python3
"""Golden vectors for geometry.overlay.boolean, from GeographicLib's C++
tools (independent of the tool's geographiclib-rs): areas by Planimeter.
In the overlap scenario each crossing is where a meridian edge meets the
middle of an edge between two corners at one latitude, so it sits at that
geodesic's vertex, found by Clairaut's relation with GeodSolve's azimuth.
Requires GeodSolve and Planimeter."""
import json
import math
import subprocess
import sys
from pathlib import Path

F = 1 / 298.257223563
SRC = "Areas by GeographicLib's Planimeter, crossings by Clairaut's relation with GeodSolve (C++), via tools/vectors/gen_overlay.py"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"


def area(pts):
    out = subprocess.run(["Planimeter", "-p", "9"], input="".join(f"{a} {b}\n" for a, b in pts), capture_output=True, text=True, check=True).stdout.split()
    return abs(float(out[2]))


def vertex_lat(lat, lon1, lon2):
    o = subprocess.run(["GeodSolve", "-i", "-p", "12"], input=f"{lat} {lon1} {lat} {lon2}\n", capture_output=True, text=True, check=True).stdout.split()
    beta1 = math.atan((1 - F) * math.tan(math.radians(lat)))
    bmax = math.acos(abs(math.sin(math.radians(float(o[0])))) * math.cos(beta1))
    return math.degrees(math.atan(math.tan(bmax) / (1 - F)))


def rect(s, w, n, e):
    return [(s, w), (s, e), (n, e), (n, w)]


def P(pts, ring=None):
    return [dict({"lat": a, "lon": b}, **({"ring": ring} if ring else {})) for a, b in pts]


def vec(i, inp, exp, src=SRC, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": tol if k.endswith("area.value") else 0, "abs": 0 if k.endswith("area.value") else 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def main():
    out = []
    a = rect(40.0, -105.0, 40.008, -104.99)
    b = rect(40.004, -104.995, 40.012, -104.985)
    # The spec scenario: two geofences overlapping in a quadrilateral.
    x1 = (vertex_lat(40.004, -104.995, -104.985), -104.99)  # a's east edge meets the middle of b's south edge
    x2 = (vertex_lat(40.008, -104.99, -105.0), -104.995)    # b's west edge meets the middle of a's north edge
    overlap = area([(40.004, -104.995), x1, (40.008, -104.99), x2])
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(b), "operation": "intersection"}, {"result.parts": 1, "result.area.value": overlap / 1e6}, SPEC))
    aa, ab = area(a), area(b)
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(b), "operation": "union"}, {"result.parts": 1, "result.area.value": (aa + ab - overlap) / 1e6}))
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(b), "operation": "difference"}, {"result.parts": 1, "result.area.value": (aa - overlap) / 1e6}))
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(b), "operation": "symmetric-difference"}, {"result.parts": 2, "result.area.value": (aa + ab - 2 * overlap) / 1e6}))
    # One inside the other.
    inner = rect(40.002, -104.998, 40.006, -104.994)
    ai = area(inner)
    for op, parts, ar in [("intersection", 1, ai), ("union", 1, aa), ("difference", 1, aa - ai)]:
        out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(inner), "operation": op}, {"result.parts": parts, "result.area.value": ar / 1e6, "result.area_b.value": ai / 1e6}))
    # Apart: no overlap, and a union in two parts.
    apart = rect(40.1, -105.0, 40.108, -104.99)
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(apart), "operation": "intersection"}, {"result.parts": 0}))
    out.append(vec(len(out) + 1, {"polygon_a": P(a), "polygon_b": P(apart), "operation": "union"}, {"result.parts": 2, "result.area.value": (aa + area(apart)) / 1e6}))
    # Across the antimeridian.
    w, e = rect(-1, 179.0, 1, -179.5), rect(-0.5, 179.5, 0.5, -179.0)
    out.append(vec(len(out) + 1, {"polygon_a": P(w), "polygon_b": P(e), "operation": "intersection"}, {"result.parts": 1}))
    out.append(vec(len(out) + 1, {"polygon_a": P(a[:2]), "polygon_b": P(b), "operation": "union"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.overlay.boolean.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
