#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.flow-direction (add-raster-hydrology,
raster/hydrology).

The oracle is a second implementation written here from the published rules:
D8 by steepest descent (O'Callaghan and Mark 1984), and D-infinity from
Tarboton (1997) equations (1) to (6) with the facet factors of his Table 1,
read in the author's copy of the paper. The three pointer encodings come from
their own documentation: ESRI's powers of two, the WhiteboxTools clockwise
table (Whitebox GAT help: a cell flowing north-west is 64), and TauDEM's 1 to
8 counterclockwise from east.

    gen_flowdir.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import random

SQRT2 = math.sqrt(2.0)
# row step, column step, ESRI, WhiteboxTools, TauDEM.
DIRS = [
    (0, 1, 1, 2, 1), (1, 1, 2, 4, 8), (1, 0, 4, 8, 7), (1, -1, 8, 16, 6),
    (0, -1, 16, 32, 5), (-1, -1, 32, 64, 4), (-1, 0, 64, 128, 3), (-1, 1, 128, 1, 2),
]
# Tarboton Table 1: e1, e2, a_c, a_f.
FACETS = [
    ((0, 1), (-1, 1), 0, 1), ((-1, 0), (-1, 1), 1, -1), ((-1, 0), (-1, -1), 1, 1),
    ((0, -1), (-1, -1), 2, -1), ((0, -1), (1, -1), 2, 1), ((1, 0), (1, -1), 3, -1),
    ((1, 0), (1, 1), 3, 1), ((0, 1), (1, 1), 4, -1),
]


def at(g, r, c):
    h, w = len(g), len(g[0])
    if 0 <= r < h and 0 <= c < w:
        return g[r][c]
    return None


def d8(g, r, c, cell, encoding):
    z0 = g[r][c]
    best = None
    for k, (dr, dc, esri, wbt, tau) in enumerate(DIRS):
        zn = at(g, r + dr, c + dc)
        if zn is None:
            continue
        dist = cell * (SQRT2 if abs(dr) + abs(dc) == 2 else 1.0)
        s = (z0 - zn) / dist
        if s > 0 and (best is None or s > best[1]):
            best = ({"esri": esri, "whitebox": wbt, "taudem": tau}[encoding], s)
    return best


def dinf(g, r, c, cell):
    e0 = g[r][c]
    best = None
    for (dr1, dc1), (dr2, dc2), ac, af in FACETS:
        e1, e2 = at(g, r + dr1, c + dc1), at(g, r + dr2, c + dc2)
        if e1 is None or e2 is None:
            continue
        s1, s2 = (e0 - e1) / cell, (e1 - e2) / cell
        rr, s = math.atan2(s2, s1), math.hypot(s1, s2)
        if rr < 0:
            rr, s = 0.0, s1
        elif rr > math.pi / 4:
            rr, s = math.pi / 4, (e0 - e2) / (cell * SQRT2)
        if s > 0 and (best is None or s > best[1]):
            best = ((af * rr + ac * math.pi / 2) % (2 * math.pi), s)
    return best


def run(grid, cell, method, encoding):
    h, w = len(grid), len(grid[0])
    resolved = off = none_out = 0
    steepest = 0.0
    codes = [[0] * w for _ in range(h)]
    angles = [[-1.0] * w for _ in range(h)]
    for r in range(h):
        for c in range(w):
            if grid[r][c] is None:
                continue
            border = (r in (0, h - 1) or c in (0, w - 1)
                      or any(at(grid, r + dr, c + dc) is None
                             for dr in (-1, 0, 1) for dc in (-1, 0, 1) if (dr, dc) != (0, 0)
                             and 0 <= r + dr < h and 0 <= c + dc < w))
            got = dinf(grid, r, c, cell) if method == "d-infinity" else d8(grid, r, c, cell, encoding)
            if got is None:
                if border:
                    off += 1
                else:
                    none_out += 1
                continue
            resolved += 1
            steepest = max(steepest, got[1])
            if method == "d-infinity":
                angles[r][c] = got[0]
                eight = d8(grid, r, c, cell, encoding)
                codes[r][c] = eight[0] if eight else 0
            else:
                codes[r][c] = got[0]
    return {"resolved": resolved, "off_grid": off, "no_outflow": none_out,
            "steepest": steepest, "codes": codes, "angles": angles}


def rows_of(grid, no_data=None):
    # repr, not %g: the tool must read the same elevations the oracle used, and
    # six significant digits is not the same number.
    def fmt(v):
        return repr(no_data) if v is None else repr(float(v))
    return [{"row": ", ".join(fmt(v) for v in row)} for row in grid]



def case(rows, grid, cell, method, encoding, source, no_data=None, check_grid=False):
    r = run(grid, cell, method, encoding)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m",
           "method": method, "encoding": encoding}
    if no_data is not None:
        inp["no_data"] = no_data
    ex = {"ok": True, "result.resolved": r["resolved"], "result.off_grid": r["off_grid"],
          "result.no_outflow": r["no_outflow"], "result.steepest": r["steepest"]}
    if check_grid:
        # Every row of the pointer grid, as the tool prints it.
        for i, row in enumerate(r["codes"]):
            # A no-data cell carries the no-data value, not a direction.
            ex[f"result.directions.{i}.row"] = ", ".join(
                repr(no_data) if grid[i][j] is None else str(v) for j, v in enumerate(row))
    if r["no_outflow"]:
        ex["meta.warnings.*.code"] = "PITS_PRESENT"
    tol = {k: {"abs": 1e-9 if k.endswith("steepest") else 0}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    rows.append({"input": inp, "expect": ex, "source": source,
                 "sourceVersion": "Python math; O'Callaghan and Mark 1984; Tarboton 1997 Table 1",
                 "tolerance": tol})


def main():
    rows = []
    SPEC = "add-raster-hydrology raster/hydrology scenario"
    PAPERS = "D8 (O'Callaghan and Mark 1984) and D-infinity (Tarboton 1997, Table 1) reimplemented in Python"

    # The Ramp scenario, in all three encodings and both methods.
    ramp = [[100.0 - c for c in range(10)] for _ in range(10)]
    for enc in ("esri", "whitebox", "taudem"):
        case(rows, ramp, 30.0, "d8", enc, SPEC + f": Ramp falling east, {enc} encoding", check_grid=True)
    case(rows, ramp, 30.0, "d-infinity", "esri", SPEC + ": Ramp falling east, D-infinity angle 0 rad")

    # Ramps the other three ways, so every cardinal code is exercised.
    for name, grid in [
        ("west", [[100.0 + c for c in range(8)] for _ in range(8)]),
        ("south", [[100.0 - r for _ in range(8)] for r in range(8)]),
        ("north", [[100.0 + r for _ in range(8)] for r in range(8)]),
    ]:
        case(rows, grid, 20.0, "d8", "esri", PAPERS + f": a ramp falling {name}", check_grid=True)

    # A diagonal plane: the steepest descent is a diagonal neighbour.
    diag = [[100.0 - r - c for c in range(7)] for r in range(7)]
    case(rows, diag, 10.0, "d8", "esri", PAPERS + ": a plane falling south-east", check_grid=True)
    case(rows, diag, 10.0, "d-infinity", "esri", PAPERS + ": the same plane by D-infinity")

    # A cone draining outward, the figure Tarboton uses: every direction appears.
    cone = [[200.0 - math.hypot(r - 7.5, c - 7.5) * 10.0 for c in range(16)] for r in range(16)]
    for method in ("d8", "d-infinity"):
        case(rows, cone, 10.0, method, "esri", PAPERS + f": an outward draining cone, {method}")
    case(rows, cone, 10.0, "d8", "taudem", PAPERS + ": the cone in TauDEM codes")

    # A pit: the interior cell has nowhere to go, and the warning says so.
    pit = [[100.0, 100.0, 100.0], [100.0, 95.0, 100.0], [100.0, 100.0, 100.0]]
    case(rows, pit, 10.0, "d8", "esri", SPEC + ": an unconditioned pit has no outflow", check_grid=True)

    # No data: a hole is a boundary, and the cells beside it drain out of the data.
    hole = [[100.0, 99.0, 98.0, 97.0], [100.0, None, 98.0, 97.0],
            [100.0, 99.0, 98.0, 97.0], [100.0, 99.0, 98.0, 97.0]]
    case(rows, hole, 10.0, "d8", "esri", PAPERS + ": a no-data hole", no_data=-9999, check_grid=True)

    # Seeded random terrain at three sizes, both methods.
    rng = random.Random(7)
    for n, method in [(8, "d8"), (11, "d8"), (14, "d8"), (8, "d-infinity"), (11, "d-infinity"), (14, "d-infinity")]:
        grid = [[round(120.0 + 10.0 * math.sin(r / 4.0) + 7.0 * math.cos(c / 3.0) + rng.uniform(-2, 2), 3)
                 for c in range(n)] for r in range(n)]
        case(rows, grid, 15.0, method, "esri", f"{PAPERS}: {n} by {n} of seeded random terrain, {method}")

    # Refusals.
    rows.append({"input": {"elevations": [{"row": "1, 2"}, {"row": "3, 4"}], "cell_size": "-5 m"},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": the cell size must be more than zero",
                 "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {"elevations": [{"row": ", ".join(["1"] * 200)} for _ in range(200)], "cell_size": "10 m"},
                 "expect": {"ok": False, "error.code": "LIMIT_EXCEEDED"},
                 "source": SPEC + ": a window of 40,000 cells is past the 25,000 the tool takes",
                 "sourceVersion": "2026", "tolerance": {}})

    for i, r in enumerate(rows, 1):
        print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
