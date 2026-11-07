#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.fill (add-raster-hydrology,
raster/hydrology).

The oracle is a second implementation of Priority-Flood + epsilon written here
from the published algorithm (Barnes, Lehman and Mulla 2014, Algorithm 3), with
Python's heapq rather than the core's binary heap, and of the hybrid breach
(Lindsay 2016) with a plain Dijkstra. It shares no code with the Rust one.

Every case also carries the invariants the spec names: after conditioning no
cell is a pit, no no-data cell moves, and the fill volume is the sum of the
rises times the cell area.

    gen_hydrology.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import heapq
import json
import math
import random

EPS = 1e-5


def neighbours(i, w, h):
    r, c = divmod(i, w)
    for dr in (-1, 0, 1):
        for dc in (-1, 0, 1):
            if dr == 0 and dc == 0:
                continue
            nr, nc = r + dr, c + dc
            if 0 <= nr < h and 0 <= nc < w:
                yield nr * w + nc


def priority_flood(z, w, h, eps=EPS):
    """Barnes et al. Algorithm 3, with the window edge and the no-data
    boundary as the outlets."""
    out = list(z)
    closed = [False] * len(z)
    heap, seq = [], 0
    for i, v in enumerate(z):
        if v is None:
            continue
        r, c = divmod(i, w)
        edge = r == 0 or c == 0 or r + 1 == h or c + 1 == w
        if edge or any(z[n] is None for n in neighbours(i, w, h)):
            closed[i] = True
            heapq.heappush(heap, (v, seq, i))
            seq += 1
    while heap:
        zc, _, i = heapq.heappop(heap)
        for n in neighbours(i, w, h):
            if closed[n] or out[n] is None:
                continue
            closed[n] = True
            if out[n] <= zc + eps:
                out[n] = zc + eps
            heapq.heappush(heap, (out[n], seq, n))
            seq += 1
    return out


def pits(z, w, h):
    """Interior cells that trap water. A cell on the edge or beside no data
    drains out of the window, so it is never a pit however low it is."""
    out = []
    for i, v in enumerate(z):
        if v is None:
            continue
        r, c = divmod(i, w)
        if r == 0 or c == 0 or r + 1 == h or c + 1 == w:
            continue
        ns = [z[n] for n in neighbours(i, w, h)]
        if any(n is None for n in ns):
            continue
        if all(n > v for n in ns):
            out.append(i)
    return out


def breach(z, w, h, pit, max_depth, max_length, eps=EPS):
    """Least-cost breaching of one pit: Dijkstra with the earth dug as cost."""
    zp = z[pit]
    cost = [math.inf] * len(z)
    came, steps = [-1] * len(z), [-1] * len(z)
    cost[pit], steps[pit] = 0.0, 0
    heap, seq, target = [(0.0, 0, pit)], 1, None
    while heap:
        c, _, i = heapq.heappop(heap)
        if c > cost[i]:
            continue
        if i != pit and (z[i] is None or z[i] < zp):
            target = i
            break
        if steps[i] >= max_length:
            continue
        for n in neighbours(i, w, h):
            dig = 0.0 if z[n] is None else max(0.0, z[n] - (zp - eps * (steps[i] + 1)))
            if dig > max_depth:
                continue
            nxt = c + dig
            if nxt < cost[n]:
                cost[n], came[n], steps[n] = nxt, i, steps[i] + 1
                heapq.heappush(heap, (nxt, seq, n))
                seq += 1
    if target is None:
        return False
    path, at = [], target
    while at != pit and at != -1:
        path.append(at)
        at = came[at]
    path.reverse()
    for k, i in enumerate(path):
        if z[i] is None:
            continue
        want = zp - eps * (k + 1)
        if z[i] > want:
            z[i] = want
    return True


def condition(grid, method, cell, max_depth=10.0, max_length=100, eps=EPS):
    h, w = len(grid), len(grid[0])
    before = [v for row in grid for v in row]
    work = list(before)
    unbreached = 0
    if method == "breach":
        for pit in pits(before, w, h):
            if not breach(work, w, h, pit, max_depth, max_length, eps):
                unbreached += 1
    after = priority_flood(work, w, h, eps)
    area = cell * cell
    fill = sum((a - b) * area for a, b in zip(after, before) if a is not None and b is not None and a > b)
    cut = sum((b - a) * area for a, b in zip(after, before) if a is not None and b is not None and a < b)
    changed = sum(1 for a, b in zip(after, before) if a is not None and b is not None and a != b)
    rises = [a - b for a, b in zip(after, before) if a is not None and b is not None and a > b]
    drops = [b - a for a, b in zip(after, before) if a is not None and b is not None and a < b]
    # The invariants the spec names, checked here on the oracle itself.
    assert not pits(after, w, h), "the oracle left a pit"
    assert all((a is None) == (b is None) for a, b in zip(after, before)), "no data moved"
    return {
        "after": after, "fill": fill, "cut": cut, "changed": changed,
        "max_raise": max(rises, default=0.0), "max_cut": max(drops, default=0.0),
        "pits_before": len(pits(before, w, h)), "pits_after": len(pits(after, w, h)),
        "unbreached": unbreached,
    }


def rows_of(grid, no_data=None):
    def fmt(v):
        return f"{no_data}" if v is None else f"{v:g}"
    return [{"row": ", ".join(fmt(v) for v in row)} for row in grid]


def case(grid, method, cell, source, no_data=None, max_depth=10.0, max_length=100, extra=None):
    eps = float((extra or {}).get("epsilon", EPS))
    r = condition(grid, method, cell, max_depth, max_length, eps)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m", "method": method}
    if no_data is not None:
        inp["no_data"] = no_data
    if extra:
        inp.update(extra)
    ex = {
        "ok": True,
        "result.changed_cells": r["changed"],
        "result.fill_volume.value": r["fill"],
        "result.cut_volume.value": r["cut"],
        "result.max_raise.value": r["max_raise"],
        "result.max_cut.value": r["max_cut"],
        "result.pits_before": r["pits_before"],
        "result.pits_after": r["pits_after"],
    }
    if r["unbreached"]:
        ex["meta.warnings.*.code"] = "BREACH_INCOMPLETE"
    # Every numeric expectation needs a tolerance: the counts are exact.
    tol = {k: {"abs": 1e-6 if k.endswith(".value") else 0}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"input": inp, "expect": ex, "source": source,
            "sourceVersion": "Python heapq; Barnes et al. 2014 Alg. 3; Lindsay 2016", "tolerance": tol}


SPEC = "add-raster-hydrology raster/hydrology scenario"
PAPER = "Priority-Flood + epsilon (Barnes, Lehman and Mulla 2014, Alg. 3) reimplemented in Python"
rows = []

# The Single bowl scenario: a 7 x 7 grid with one pit 3 m below its spill.
bowl = [[100.0] * 7,
        [100.0, 99.0, 99.0, 99.0, 99.0, 99.0, 100.0],
        [100.0, 99.0, 98.0, 98.0, 98.0, 99.0, 100.0],
        [100.0, 99.0, 98.0, 95.0, 98.0, 99.0, 100.0],
        [100.0, 99.0, 98.0, 98.0, 98.0, 99.0, 100.0],
        [100.0, 99.0, 99.0, 99.0, 99.0, 99.0, 100.0],
        [100.0] * 7]
rows.append(case(bowl, "fill", 10.0, SPEC + ": Single bowl, one interior pit"))
rows.append(case(bowl, "breach", 10.0, "Lindsay 2016's hybrid on the Single bowl grid, reimplemented in Python"))

# The Pit at the edge stays scenario: the depression's lowest cell is an edge.
edge = [[100.0, 99.0, 98.0, 97.0, 96.0],
        [100.0, 99.0, 98.0, 97.0, 96.0],
        [100.0, 99.0, 98.0, 97.0, 95.0],
        [100.0, 99.0, 98.0, 97.0, 96.0],
        [100.0, 99.0, 98.0, 97.0, 96.0]]
rows.append(case(edge, "fill", 30.0, SPEC + ": Pit at the edge stays, because the edge is an outlet"))

# A grid with no data: the hole is a boundary, and never moves.
hole = [[100.0, 100.0, 100.0, 100.0, 100.0],
        [100.0, 99.0, 98.0, 99.0, 100.0],
        [100.0, 98.0, None, 98.0, 100.0],
        [100.0, 99.0, 97.0, 99.0, 100.0],
        [100.0, 100.0, 100.0, 100.0, 100.0]]
rows.append(case(hole, "fill", 10.0, SPEC + ": no-data cells are never filled and never crossed", no_data=-9999))

# A plane that already drains: nothing moves.
plane = [[100.0 - 0.5 * c - 0.1 * r for c in range(8)] for r in range(8)]
rows.append(case(plane, "fill", 25.0, PAPER + ": a plane that already drains is left alone"))

# Nested depressions: a bowl inside a bowl.
nested = [[100.0] * 9,
          [100.0, 98.0, 98.0, 98.0, 98.0, 98.0, 98.0, 98.0, 100.0],
          [100.0, 98.0, 96.0, 96.0, 96.0, 96.0, 96.0, 98.0, 100.0],
          [100.0, 98.0, 96.0, 94.0, 94.0, 94.0, 96.0, 98.0, 100.0],
          [100.0, 98.0, 96.0, 94.0, 90.0, 94.0, 96.0, 98.0, 100.0],
          [100.0, 98.0, 96.0, 94.0, 94.0, 94.0, 96.0, 98.0, 100.0],
          [100.0, 98.0, 96.0, 96.0, 96.0, 96.0, 96.0, 98.0, 100.0],
          [100.0, 98.0, 98.0, 98.0, 98.0, 98.0, 98.0, 98.0, 100.0],
          [100.0] * 9]
rows.append(case(nested, "fill", 10.0, PAPER + ": nested depressions, filled from the outside in"))
rows.append(case(nested, "breach", 10.0, "Lindsay 2016's hybrid on nested depressions, reimplemented in Python"))

# A dam across a valley: breaching cuts through it, filling does not.
# A basin held back by a ridge, draining south over the bottom edge: filling
# raises the basin to the ridge's spill, breaching cuts through the ridge.
dam = [[60.0, 60.0, 60.0, 60.0, 60.0],
       [60.0, 50.0, 48.0, 50.0, 60.0],
       [60.0, 55.0, 54.0, 55.0, 60.0],
       [60.0, 40.0, 38.0, 40.0, 60.0],
       [60.0, 30.0, 20.0, 30.0, 60.0]]
rows.append(case(dam, "fill", 20.0, PAPER + ": a dam across a valley, filled"))
rows.append(case(dam, "breach", 20.0, "Lindsay 2016's hybrid: the dam is cut through rather than the valley filled"))
rows.append(case(dam, "breach", 20.0, "Lindsay 2016's hybrid with a cut too shallow to breach the dam",
                 max_depth=0.5, extra={"max_breach_depth": "0.5 m"}))

# Random terrain with pits, at three sizes and two methods.
rng = random.Random(42)
for n, method in [(6, "fill"), (9, "fill"), (12, "fill"), (15, "fill"),
                  (6, "breach"), (9, "breach"), (12, "breach"), (15, "breach")]:
    grid = [[round(100.0 + 8.0 * math.sin(r / 3.0) + 6.0 * math.cos(c / 2.5) + rng.uniform(-3, 3), 3)
             for c in range(n)] for r in range(n)]
    rows.append(case(grid, method, 10.0, f"{PAPER}: {n} by {n} of seeded random terrain, method {method}"))

# A coarser slope per cell, and one of zero.
rows.append(case(bowl, "fill", 10.0, PAPER + ": a slope of 0.01 m a cell", extra={"epsilon": 0.01}))
rows.append(case(bowl, "fill", 10.0, PAPER + ": a slope of zero, which leaves flats", extra={"epsilon": 0.0}))

# Refusals.
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
