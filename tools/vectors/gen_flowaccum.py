#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.flow-accumulation (add-raster-hydrology,
raster/hydrology).

The oracle is a second implementation written here: D8 accumulation by walking
every cell's path downhill to exhaustion (a different method from the core's
topological sweep, so the two agree only if both are right), and D-infinity by
Tarboton's proportional split between the two neighbours the angle lies
between, summed from the highest cell down.

    gen_flowaccum.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import random

from gen_flowdir import DIRS, at, d8, dinf, rows_of


def accumulate_d8(grid, cell):
    """Each cell's water walked downhill one step at a time, which needs no
    ordering at all: the core sums in topological order instead."""
    h, w = len(grid), len(grid[0])
    acc = [[0.0] * w for _ in range(h)]
    step = {esri: (dr, dc) for dr, dc, esri, _, _ in DIRS}
    for r in range(h):
        for c in range(w):
            if grid[r][c] is None:
                continue
            rr, cc, seen = r, c, 0
            while True:
                acc[rr][cc] += 1.0
                got = d8(grid, rr, cc, cell, "esri")
                if got is None:
                    break
                dr, dc = step[got[0]]
                rr, cc = rr + dr, cc + dc
                seen += 1
                if seen > h * w:
                    raise AssertionError("the oracle found a loop")
    return acc


def accumulate_dinf(grid, cell):
    h, w = len(grid), len(grid[0])
    acc = [[1.0 if grid[r][c] is not None else 0.0 for c in range(w)] for r in range(h)]
    order = sorted(((r, c) for r in range(h) for c in range(w) if grid[r][c] is not None),
                   key=lambda rc: -grid[rc[0]][rc[1]])
    ccw = {k: DIRS[(8 - k) % 8] for k in range(9)}
    for r, c in order:
        got = dinf(grid, r, c, cell)
        if got is None:
            continue
        angle = got[0]
        k = math.floor(angle / (math.pi / 4))
        frac = angle / (math.pi / 4) - k
        for (dr, dc, *_), share in ((ccw[int(k) % 8], 1 - frac), (ccw[(int(k) + 1) % 8], frac)):
            if share <= 0:
                continue
            if at(grid, r + dr, c + dc) is None:
                continue
            acc[r + dr][c + dc] += acc[r][c] * share
    return acc


def js(v):
    """A number as the core serializes it: ECMAScript's shortest round trip,
    so a whole number carries no decimal point."""
    return str(int(v)) if v == int(v) and abs(v) < 1e21 else repr(v)


rows = []


def case(grid, cell, method, source, no_data=None, check_grid=False):
    acc = accumulate_dinf(grid, cell) if method == "d-infinity" else accumulate_d8(grid, cell)
    h, w = len(grid), len(grid[0])
    best, at_rc = 0.0, (1, 1)
    for r in range(h):
        for c in range(w):
            if grid[r][c] is not None and acc[r][c] > best:
                best, at_rc = acc[r][c], (r + 1, c + 1)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m", "method": method}
    if no_data is not None:
        inp["no_data"] = no_data
    ex = {"ok": True, "result.max_cells": round(best, 2),
          "result.max_area.value": best * cell * cell,
          "result.outlet_row": at_rc[0], "result.outlet_column": at_rc[1]}
    if check_grid:
        for i, row in enumerate(acc):
            ex[f"result.accumulation.{i}.row"] = ", ".join(
                js(no_data) if grid[i][j] is None else js(v) for j, v in enumerate(row))
    # A pit traps what reaches it; the tool warns, so the vector expects it.
    def is_pit(r, c):
        if grid[r][c] is None or r in (0, h - 1) or c in (0, w - 1):
            return False
        ns = [at(grid, r + dr, c + dc) for dr in (-1, 0, 1) for dc in (-1, 0, 1) if (dr, dc) != (0, 0)]
        return None not in ns and all(n > grid[r][c] for n in ns)
    if any(is_pit(r, c) for r in range(h) for c in range(w)):
        ex["meta.warnings.*.code"] = "PITS_PRESENT"
    tol = {k: {"abs": 1e-9 if k.endswith(".value") or k.endswith("max_cells") else 0}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    rows.append({"input": inp, "expect": ex, "source": source,
                 "sourceVersion": "Python math; D8 walked downhill; Tarboton 1997 split",
                 "tolerance": tol})


SPEC = "add-raster-hydrology raster/hydrology scenario"
PAPERS = "D8 accumulation walked downhill, and Tarboton's (1997) split, reimplemented in Python"

ramp = [[100.0 - c for c in range(10)] for _ in range(10)]
case(ramp, 30.0, "d8", SPEC + ": East ramp accumulation, each row 1 to 10", check_grid=True)
case(ramp, 30.0, "d-infinity", SPEC + ": the same ramp by D-infinity, which has nothing to split", check_grid=True)

south = [[100.0 - r for _ in range(6)] for r in range(6)]
case(south, 20.0, "d8", PAPERS + ": a ramp falling south", check_grid=True)

diag = [[100.0 - r - c for c in range(7)] for r in range(7)]
case(diag, 10.0, "d8", PAPERS + ": a plane falling south-east", check_grid=True)
case(diag, 10.0, "d-infinity", PAPERS + ": the same plane by D-infinity")

valley = [[100.0 - r + abs(c - 4) * 2.0 for c in range(9)] for r in range(9)]
case(valley, 25.0, "d8", PAPERS + ": a valley draining south, which gathers along its floor", check_grid=True)
case(valley, 25.0, "d-infinity", PAPERS + ": the same valley by D-infinity")

cone = [[200.0 - math.hypot(r - 7.5, c - 7.5) * 10.0 for c in range(16)] for r in range(16)]
case(cone, 10.0, "d8", PAPERS + ": an outward draining cone, where every cell keeps its own water")
case(cone, 10.0, "d-infinity", PAPERS + ": the cone by D-infinity")

pit = [[100.0, 100.0, 100.0], [100.0, 95.0, 100.0], [100.0, 100.0, 100.0]]
case(pit, 10.0, "d8", SPEC + ": a pit swallows everything that reaches it", check_grid=True)

hole = [[100.0, 99.0, 98.0, 97.0], [100.0, None, 98.0, 97.0],
        [100.0, 99.0, 98.0, 97.0], [100.0, 99.0, 98.0, 97.0]]
case(hole, 10.0, "d8", PAPERS + ": a no-data hole takes no water and gives none", no_data=-9999, check_grid=True)

rng = random.Random(11)
for n, method in [(8, "d8"), (10, "d8"), (13, "d8"), (8, "d-infinity"), (10, "d-infinity"), (13, "d-infinity")]:
    grid = [[round(150.0 - r * 2.0 + 6.0 * math.cos(c / 2.5) + rng.uniform(-1.5, 1.5), 3)
             for c in range(n)] for r in range(n)]
    case(grid, 15.0, method, f"{PAPERS}: {n} by {n} of seeded random terrain, {method}")

rows.append({"input": {"elevations": [{"row": "1, 2"}, {"row": "3, 4"}], "cell_size": "0 m"},
             "expect": {"ok": False, "error.code": "INVALID_INPUT"},
             "source": SPEC + ": the cell size must be more than zero",
             "sourceVersion": "2026", "tolerance": {}})
rows.append({"input": {"elevations": [{"row": ", ".join(["1"] * 200)} for _ in range(200)], "cell_size": "10 m"},
             "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED"},
             "source": SPEC + ": a window of 40,000 cells is past the 25,000 the tool takes",
             "sourceVersion": "2026", "tolerance": {}})

for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
