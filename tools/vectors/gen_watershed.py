#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.watershed (add-raster-hydrology,
raster/hydrology).

The oracle is a second implementation written here: the basin found by walking
every cell's own path downhill and keeping the cells whose path passes the
outlet (the core walks the pointers upstream from the outlet instead), the
longest flow path by the same walk, and the snapping rules of Jenson and
Domingue (1988) as the change's design states them.

    gen_watershed.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import random

from gen_flowdir import DIRS, at, d8, rows_of
from gen_flowaccum import accumulate_d8

SQRT2 = math.sqrt(2.0)
STEP = {esri: (dr, dc) for dr, dc, esri, _, _ in DIRS}


def path_down(grid, cell, start):
    """Every cell from `start` down to where the water leaves, with the length
    walked to each one."""
    out, length, (r, c) = [(start, 0.0)], 0.0, start
    seen = {start}
    while True:
        got = d8(grid, r, c, cell, "esri")
        if not got:
            break
        dr, dc = STEP[got[0]]
        length += cell * (SQRT2 if abs(dr) + abs(dc) == 2 else 1.0)
        r, c = r + dr, c + dc
        if (r, c) in seen:
            raise AssertionError("the oracle found a loop")
        seen.add((r, c))
        out.append(((r, c), length))
    return out


def watershed(grid, cell, outlet):
    h, w = len(grid), len(grid[0])
    basin, longest, longest_from = set(), 0.0, outlet
    for r in range(h):
        for c in range(w):
            if grid[r][c] is None:
                continue
            walk = path_down(grid, cell, (r, c))
            for rc, length in walk:
                if rc == outlet:
                    basin.add((r, c))
                    if length > longest:
                        longest, longest_from = length, (r, c)
                    break
    high = max(grid[r][c] for r, c in basin)
    slopes = [d8(grid, r, c, cell, "esri") for r, c in basin]
    slopes = [s[1] for s in slopes if s]
    touches = any(r in (0, h - 1) or c in (0, w - 1) for r, c in basin)
    drop = grid[longest_from[0]][longest_from[1]] - grid[outlet[0]][outlet[1]]
    return {"cells": len(basin), "relief": high - grid[outlet[0]][outlet[1]],
            "mean_slope": sum(slopes) / len(slopes) if slopes else 0.0,
            "longest": longest, "drop": drop, "touches": touches,
            "basin": basin}


def snap(grid, cell, at_rc, rule, threshold, within):
    if rule == "none":
        return at_rc
    h, w = len(grid), len(grid[0])
    acc = accumulate_d8(grid, cell)
    best = None
    for dr in range(-within, within + 1):
        for dc in range(-within, within + 1):
            r, c = at_rc[0] + dr, at_rc[1] + dc
            if not (0 <= r < h and 0 <= c < w) or grid[r][c] is None:
                continue
            d2 = dr * dr + dc * dc
            if rule == "highest-accumulation":
                if best is None or acc[r][c] > best[1]:
                    best = ((r, c), acc[r][c], d2)
            elif acc[r][c] >= threshold and (best is None or d2 < best[2]
                                             or (d2 == best[2] and acc[r][c] > best[1])):
                best = ((r, c), acc[r][c], d2)
    return best[0] if best else at_rc


def case(rows, grid, cell, pour, source, rule="none", threshold=None, within=3,
         no_data=None, check_basin=False):
    h, w = len(grid), len(grid[0])
    thr = threshold if threshold is not None else max(round(h * w * 0.001), 5)
    outlet = snap(grid, cell, (pour[0] - 1, pour[1] - 1), rule, thr, within)
    r = watershed(grid, cell, outlet)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m",
           "pour_row": pour[0], "pour_column": pour[1], "snap": rule}
    if threshold is not None:
        inp["threshold_cells"] = threshold
    if no_data is not None:
        inp["no_data"] = no_data
    ex = {"ok": True, "result.basin_cells": r["cells"],
          "result.basin_area.value": r["cells"] * cell * cell,
          "result.outlet_row": outlet[0] + 1, "result.outlet_column": outlet[1] + 1,
          "result.relief.value": r["relief"], "result.mean_slope": round(r["mean_slope"], 4),
          "result.longest_path.value": r["longest"], "result.path_drop.value": r["drop"],
          "result.touches_edge": "yes" if r["touches"] else "no"}
    if check_basin:
        for i in range(h):
            ex[f"result.basin.{i}.row"] = ", ".join(
                f"{no_data}" if grid[i][j] is None else ("1" if (i, j) in r["basin"] else "0")
                for j in range(w))
    tol = {k: {"abs": 1e-9 if k.endswith(".value") else (1e-4 if k.endswith("mean_slope") else 0)}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    rows.append({"input": inp, "expect": ex, "source": source,
                 "sourceVersion": "Python math; Jenson and Domingue 1988; D8 paths walked downhill",
                 "tolerance": tol})


def main():
    rows = []
    SPEC = "add-raster-hydrology raster/hydrology scenario"
    PAPERS = "Jenson and Domingue's (1988) basin, reimplemented in Python by walking every cell downhill"

    fork = [[60.0, 48.0, 60.0, 60.0, 60.0, 48.0, 60.0],
            [60.0, 60.0, 47.0, 60.0, 47.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 46.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 45.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 44.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 43.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 42.0, 60.0, 60.0, 60.0]]
    case(rows, fork, 30.0, (7, 4), "The valley's outlet, unsnapped", check_basin=True)
    case(rows, fork, 30.0, (4, 4), "A cell upstream of it: the nested-basin scenario", check_basin=True)
    case(rows, fork, 30.0, (7, 3), "A point one cell off the channel, snapped to the nearest stream cell",
         rule="nearest-stream", threshold=5)
    case(rows, fork, 30.0, (7, 3), "The same point snapped to the most water within reach",
         rule="highest-accumulation")

    # The Ramp path scenario: the longest path at the east edge of a 10-cell
    # east ramp with 30 m cells is 270 m.
    ramp = [[100.0 - c for c in range(10)] for _ in range(10)]
    case(rows, ramp, 30.0, (1, 10), SPEC + ": the Ramp path, 270 m along the row")
    case(rows, ramp, 30.0, (5, 10), SPEC + ": the same ramp, from the middle row")

    valley = [[100.0 - r + abs(c - 5) * 3.0 for c in range(11)] for r in range(11)]
    for pour in ((11, 6), (8, 6), (6, 6)):
        case(rows, valley, 20.0, pour, PAPERS + f": a valley draining south, outlet at {pour}")

    cone = [[200.0 - math.hypot(r - 7.5, c - 7.5) * 10.0 for c in range(16)] for r in range(16)]
    case(rows, cone, 10.0, (1, 8), PAPERS + ": an outward draining cone, outlet on its north edge")

    hole = [[100.0 - r + abs(c - 3) * 2.0 for c in range(7)] for r in range(7)]
    hole[3][5] = None
    case(rows, hole, 15.0, (7, 4), PAPERS + ": a valley with a no-data hole beside it",
         no_data=-9999, check_basin=True)

    rng = random.Random(31)
    for n in (9, 12):
        grid = [[round(140.0 - r * 1.5 + 5.0 * math.cos(c / 2.0) + rng.uniform(-1.0, 1.0), 3)
                 for c in range(n)] for r in range(n)]
        for pour in ((n, n // 2), (n - 2, 2)):
            case(rows, grid, 25.0, pour, f"{PAPERS}: {n} by {n} of seeded random terrain, outlet at {pour}")

    base = {"elevations": rows_of(fork), "cell_size": "30.0 m"}
    rows.append({"input": {**base, "pour_row": 99, "pour_column": 4},
                 "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                 "source": SPEC + ": a pour point off the window names the window's size",
                 "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {**base, "pour_column": 4},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": the pour point needs both its row and its column",
                 "sourceVersion": "2026", "tolerance": {}})
    hole_rows = rows_of(hole, -9999)
    rows.append({"input": {"elevations": hole_rows, "cell_size": "15.0 m", "no_data": -9999,
                           "pour_row": 4, "pour_column": 6},
                 "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                 "source": SPEC + ": a pour point on a no-data cell is refused",
                 "sourceVersion": "2026", "tolerance": {}})

    for i, r in enumerate(rows, 1):
        print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
