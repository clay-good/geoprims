#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.longest-flow-path
(add-raster-hydrology, raster/hydrology).

The oracle walks every cell's own path downhill and keeps the longest one that
passes the outlet, where the core walks the pointers upstream from the outlet
and keeps the longest branch above each cell.

    gen_longestpath.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import random

from gen_flowdir import rows_of
from gen_watershed import path_down


def longest(grid, cell, outlet):
    h, w = len(grid), len(grid[0])
    best, start = 0.0, outlet
    for r in range(h):
        for c in range(w):
            if grid[r][c] is None:
                continue
            for rc, length in path_down(grid, cell, (r, c)):
                if rc == outlet:
                    if length > best:
                        best, start = length, (r, c)
                    break
    # The path itself, from that start down to the outlet.
    cells = []
    for rc, _ in path_down(grid, cell, start):
        cells.append(rc)
        if rc == outlet:
            break
    drop = grid[start[0]][start[1]] - grid[outlet[0]][outlet[1]]
    return {"length": best, "drop": drop, "cells": len(cells), "start": start}


def case(rows, grid, cell, pour, source, no_data=None, check_path=False):
    outlet = (pour[0] - 1, pour[1] - 1)
    r = longest(grid, cell, outlet)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m",
           "pour_row": pour[0], "pour_column": pour[1]}
    if no_data is not None:
        inp["no_data"] = no_data
    ex = {"ok": True, "result.length.value": r["length"], "result.drop.value": r["drop"],
          "result.cells": r["cells"], "result.start_row": r["start"][0] + 1,
          "result.start_column": r["start"][1] + 1,
          "result.average_slope": round(r["drop"] / r["length"], 5) if r["length"] else 0.0}
    if check_path:
        cells = []
        for rc, _ in path_down(grid, cell, r["start"]):
            cells.append(rc)
            if rc == outlet:
                break
        for k, (rr, cc) in enumerate(cells):
            ex[f"result.path.{k}.row"] = rr + 1
            ex[f"result.path.{k}.column"] = cc + 1
    tol = {k: {"abs": 1e-9 if k.endswith(".value") else (1e-5 if k.endswith("average_slope") else 0)}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    rows.append({"input": inp, "expect": ex, "source": source,
                 "sourceVersion": "Python math; D8 paths walked downhill",
                 "tolerance": tol})


def main():
    rows = []
    SPEC = "add-raster-hydrology raster/hydrology scenario"
    PAPER = "The longest D8 chain, reimplemented in Python by walking every cell downhill"

    ramp = [[100.0 - c for c in range(10)] for _ in range(10)]
    case(rows, ramp, 30.0, (1, 10), SPEC + ": the Ramp path, 270 m along the row", check_path=True)
    case(rows, ramp, 30.0, (6, 10), SPEC + ": the same ramp from another row")

    diag = [[100.0 - r - c for c in range(8)] for r in range(8)]
    case(rows, diag, 20.0, (8, 8), PAPER + ": a plane falling south-east, where the path runs diagonally",
         check_path=True)

    fork = [[60.0, 48.0, 60.0, 60.0, 60.0, 48.0, 60.0],
            [60.0, 60.0, 47.0, 60.0, 47.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 46.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 45.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 44.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 43.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 42.0, 60.0, 60.0, 60.0]]
    case(rows, fork, 30.0, (7, 4), PAPER + ": the valley's foot, reached from one of the headwaters",
         check_path=True)
    case(rows, fork, 30.0, (3, 4), PAPER + ": the junction itself")

    valley = [[100.0 - r + abs(c - 5) * 3.0 for c in range(11)] for r in range(11)]
    for pour in ((11, 6), (7, 6)):
        case(rows, valley, 20.0, pour, PAPER + f": a valley draining south, outlet at {pour}")

    cone = [[200.0 - math.hypot(r - 7.5, c - 7.5) * 10.0 for c in range(16)] for r in range(16)]
    case(rows, cone, 10.0, (1, 8), PAPER + ": an outward draining cone")

    hole = [[100.0 - r + abs(c - 3) * 2.0 for c in range(7)] for r in range(7)]
    hole[3][5] = None
    case(rows, hole, 15.0, (7, 4), PAPER + ": a valley with a no-data hole beside it", no_data=-9999)

    rng = random.Random(37)
    for n in (9, 12, 14):
        grid = [[round(140.0 - r * 1.5 + 5.0 * math.cos(c / 2.0) + rng.uniform(-1.0, 1.0), 3)
                 for c in range(n)] for r in range(n)]
        case(rows, grid, 25.0, (n, n // 2), f"{PAPER}: {n} by {n} of seeded random terrain")

    base = {"elevations": rows_of(fork), "cell_size": "30.0 m"}
    # Past the last row: outside the window. (A row of 0 is below the field's
    # own minimum, so it is an invalid input rather than a domain refusal.)
    rows.append({"input": {**base, "pour_row": 99, "pour_column": 4},
                 "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                 "source": SPEC + ": an outlet off the window", "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {**base, "pour_row": 0, "pour_column": 4},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": a row of zero is below the field's minimum of 1",
                 "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {**base, "pour_row": 7, "pour_column": 4, "north_west_lat": "44.12 deg"},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": a corner needs both its latitude and its longitude",
                 "sourceVersion": "2026", "tolerance": {}})

    for i, r in enumerate(rows, 1):
        print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
