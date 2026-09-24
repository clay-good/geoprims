#!/usr/bin/env python3
"""Contours against contourpy, for raster.terrain.contours.

contourpy is the contouring engine behind Matplotlib, written separately in
C++. Its "serial" algorithm with corner_mask off follows the same conventions
the tool documents: linear interpolation along grid lines, a saddle decided by
the mean of the square's four corners, and a square with a missing corner left
out. So for every level the two must find the same lines: the same number,
the same closed loops, the same vertex count, and the same length.

Writes:
- core/crates/gp-raster/tests/data/contours_contourpy.json: 250 random grids
  (smooth hills and pits made of sums of Gaussians, 4 to 20 points a side,
  some with no-data holes, 40% rounded to whole meters with whole-meter
  intervals so grid points often sit exactly on a level), with contourpy's
  per-level summary.
- core/vectors/raster.terrain.contours.jsonl: golden vectors.

Requires numpy and contourpy."""
import json
import math
import random
import sys
from pathlib import Path

import contourpy
import numpy as np

VER = f"contourpy {contourpy.__version__}"
SRC = "contourpy (serial, corner_mask off), via tools/vectors/gen_contours.py"
NO_DATA = -9999.0
STATS = {"changed": 0, "levels": 0}


def summary(z, cell, interval, base=0.0):
    """contourpy's lines per level: count, closed loops, vertices, length."""
    zm = np.ma.masked_equal(np.array(z, dtype=float), NO_DATA)
    gen = contourpy.contour_generator(z=zm, name="serial", corner_mask=False, line_type="Separate")
    lo, hi = float(zm.min()), float(zm.max())
    out = []
    k = math.ceil((lo - base) / interval)
    while base + k * interval <= hi:
        level = base + k * interval
        k += 1
        # contourpy returns a one-point "line" where a corner sits exactly on
        # the level with nothing lower around it (the grid's lowest points at
        # the lowest level); the tool draws no line there, so neither counts.
        lines = [l for l in gen.lines(level) if len(l) >= 2 and np.hypot(*np.diff(l, axis=0).T).sum() > 0]
        if not lines:
            continue
        n = len(lines)
        lines = untangled(joined(lines))
        STATS["levels"] += 1
        STATS["changed"] += len(lines) != n
        length = sum(float(np.hypot(*np.diff(l, axis=0).T).sum()) for l in lines) * cell
        closed = sum(1 for l in lines if len(l) > 2 and (l[0] == l[-1]).all())
        out.append({"level": level, "lines": len(lines), "closed": closed,
                    # Where a line passes exactly through a grid point contourpy
                    # repeats the point (a step of zero length); the tool does
                    # not, so repeats are not counted.
                    "vertices": sum(1 + int((np.diff(l, axis=0) != 0).any(axis=1).sum()) for l in lines), "length": length})
    return out


def joined(lines):
    """Lines joined where one ends exactly where another starts. At a grid
    point that sits exactly on the level (most often beside a no-data square)
    contourpy can restart a line, splitting one continuous contour in two; the
    tool keeps it whole. The generator prints how many levels this changes."""
    lines = [l.copy() for l in lines]
    merged = True
    while merged:
        merged = False
        for a in range(len(lines)):
            for b in range(len(lines)):
                if a != b and (lines[a][-1] == lines[b][0]).all() and not (lines[b][0] == lines[b][-1]).all():
                    lines[a] = np.vstack([lines[a], lines[b][1:]])
                    del lines[b]
                    merged = True
                    break
            if merged:
                break
    return lines


def untangled(lines):
    """Each line that passes through one point twice split into the loop
    between the visits and the rest, the rule the tool uses. Where two contours
    touch at a grid point exactly on the level, either reading is right, and
    contourpy takes one or the other depending on where its trace began."""
    out = []
    for l in lines:
        pts = []
        for p in map(tuple, l):
            if not pts or pts[-1] != p:  # contourpy's zero-length steps
                pts.append(p)
        while True:
            seen, cut = {}, None
            for j, p in enumerate(pts):
                if p in seen and not (seen[p] == 0 and j == len(pts) - 1):
                    cut = (seen[p], j)
                    break
                seen[p] = j
            if cut is None:
                break
            i, j = cut
            out.append(np.array(pts[i:j + 1]))
            del pts[i + 1:j + 1]
        out.append(np.array(pts))
    return out


def surface(rng, rows, cols):
    bumps = [(rng.uniform(0, rows), rng.uniform(0, cols), rng.uniform(-40, 60), rng.uniform(2, 8)) for _ in range(rng.randint(1, 5))]
    z = []
    for i in range(rows):
        row = []
        for j in range(cols):
            v = 100.0 + sum(h * math.exp(-((i - a) ** 2 + (j - b) ** 2) / (2 * s * s)) for a, b, h, s in bumps)
            row.append(round(v, 3))
        z.append(row)
    return z


def text_rows(z):
    return [{"row": ", ".join(repr(v) if v != NO_DATA else "-9999" for v in r)} for r in z]


def fixture(n=250, seed=20260924):
    rng = random.Random(seed)
    out = []
    for _ in range(n):
        rows, cols = rng.randint(4, 20), rng.randint(4, 20)
        z = surface(rng, rows, cols)
        if rng.random() < 0.3:
            for _ in range(rng.randint(1, 6)):
                z[rng.randrange(rows)][rng.randrange(cols)] = NO_DATA
        interval = rng.choice([1.1, 2.5, 5.3, 7.0])
        if rng.random() < 0.4:
            # Whole-meter elevations and intervals, as a real DEM export has:
            # many grid points sit exactly on a level.
            z = [[v if v == NO_DATA else float(round(v)) for v in r] for r in z]
            interval = float(rng.choice([1, 2, 5]))
        cell = rng.choice([1.0, 10.0, 30.0])
        levels = summary(z, cell, interval)
        out.append({"elevations": text_rows(z), "interval": interval, "cell_size": cell, "no_data": NO_DATA, "levels": levels})
    return out


def vec(i, inp, exp, src=SRC, rel=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": rel, "abs": 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    fx = fixture()
    (root / "core/crates/gp-raster/tests/data").mkdir(parents=True, exist_ok=True)
    (root / "core/crates/gp-raster/tests/data/contours_contourpy.json").write_text(json.dumps(fx, separators=(",", ":")) + "\n")

    out = []

    def add(z, interval, cell, base=0.0, no_data=None, src=SRC):
        inp = {"elevations": text_rows(z), "interval": f"{interval} m", "cell_size": f"{cell} m"}
        if base:
            inp["base"] = f"{base} m"
        if no_data is not None:
            inp["no_data"] = no_data
        s = summary(z, cell, interval, base)
        exp = {"result.line_count": sum(l["lines"] for l in s), "result.level_count": len(s),
               "result.length.value": sum(l["length"] for l in s)}
        for i, l in enumerate(s[:4]):
            exp[f"result.levels.{i}.level.value"] = l["level"]
            exp[f"result.levels.{i}.lines"] = l["lines"]
            exp[f"result.levels.{i}.closed"] = l["closed"]
            exp[f"result.levels.{i}.length.value"] = l["length"]
        out.append(vec(len(out) + 1, inp, exp, src))

    hill = [[100, 102, 104, 102, 100], [102, 106, 110, 106, 102], [104, 110, 118, 110, 104],
            [102, 106, 110, 106, 102], [100, 102, 104, 102, 100]]
    # The primary example. 110 m falls on grid points; contourpy and the
    # tool both put the loop through them.
    add(hill, 5, 10)
    add(hill, 3, 10)
    add(hill, 2.5, 30, base=1.25)
    pit = [[120 - (v - 100) for v in r] for r in hill]
    add(pit, 5, 10)
    # A saddle: two highs on one diagonal, two lows on the other.
    saddle = [[110, 100, 90], [100, 100.4, 100], [90, 100, 110]]
    add(saddle, 0.7, 1, base=0.15)
    add([[1.0, 0.0], [0.0, 1.0]], 0.4, 1)
    add([[1.0, 0.0], [0.0, 1.0]], 0.6, 1)
    # A plane sloping east: straight, parallel open lines.
    add([[100 + 3.3 * j for j in range(8)] for _ in range(6)], 2, 5)
    # A hole of no data breaks a loop into open lines.
    holed = [r[:] for r in hill]
    holed[1][2] = NO_DATA
    add(holed, 5, 10, no_data=NO_DATA)
    rng = random.Random(4)
    while len(out) < 22:
        z = surface(rng, rng.randint(6, 16), rng.randint(6, 16))
        interval = rng.choice([1.3, 2.7, 4.1])
        vals = [v for r in z for v in r]
        if any(abs(v / interval - round(v / interval)) < 1e-9 for v in vals):
            continue
        add(z, interval, rng.choice([1, 10, 30]))
    # Errors.
    out.append(vec(len(out) + 1, {"elevations": [{"row": "1, 2"}, {"row": "1, 2, 3"}], "interval": "1 m", "cell_size": "1 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"elevations": text_rows(hill), "interval": "0.01 m", "cell_size": "1 m"}, {"ok": False, "error.code": "LIMIT_EXCEEDED"}))
    out.append(vec(len(out) + 1, {"elevations": text_rows(hill), "interval": "0 m", "cell_size": "1 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    (root / "core/vectors/raster.terrain.contours.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    print(len(fx), "fixture grids,", len(out), "vectors;", STATS["changed"], "of", STATS["levels"], "reference levels regrouped at touching grid points")


if __name__ == "__main__":
    main()
