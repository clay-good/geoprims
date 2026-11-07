#!/usr/bin/env python3
"""Golden vectors for raster.hydrology.streams (add-raster-hydrology,
raster/hydrology).

The oracle is a second implementation written here: channel cells from the
accumulation oracle of gen_flowaccum.py, Strahler order (1957) worked out by
repeatedly ordering whatever has all its tributaries already ordered, and
links walked from each source or junction down to the next one.

    gen_streams.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import math
import random

from gen_flowdir import DIRS, at, d8, rows_of
from gen_flowaccum import accumulate_d8, js

SQRT2 = math.sqrt(2.0)
STEP = {esri: (dr, dc) for dr, dc, esri, _, _ in DIRS}


def network(grid, cell, threshold):
    h, w = len(grid), len(grid[0])
    acc = accumulate_d8(grid, cell)
    chan = {(r, c) for r in range(h) for c in range(w)
            if grid[r][c] is not None and acc[r][c] >= threshold}
    to, up = {}, {rc: 0 for rc in chan}
    for (r, c) in chan:
        got = d8(grid, r, c, cell, "esri")
        if not got:
            continue
        dr, dc = STEP[got[0]]
        if (r + dr, c + dc) in chan:
            to[(r, c)] = (r + dr, c + dc)
            up[(r + dr, c + dc)] += 1
    # Strahler by repeated passes: order whatever has every tributary ordered.
    order = {}
    while len(order) < len(chan):
        progress = False
        for rc in chan:
            if rc in order:
                continue
            ins = [order[u] for u in chan if to.get(u) == rc and u in order]
            if len(ins) != up[rc]:
                continue
            if not ins:
                order[rc] = 1
            else:
                top = max(ins)
                order[rc] = top + 1 if ins.count(top) >= 2 else top
            progress = True
        if not progress:
            raise AssertionError("the oracle found a cycle")
    heads = sorted(rc for rc in chan if up[rc] == 0 or up[rc] >= 2)
    links = []
    for head in heads:
        at_rc, length, cells = head, 0.0, 1
        while True:
            nxt = to.get(at_rc)
            if nxt is None:
                break
            dr, dc = nxt[0] - at_rc[0], nxt[1] - at_rc[1]
            length += cell * (SQRT2 if abs(dr) + abs(dc) == 2 else 1.0)
            at_rc, cells = nxt, cells + 1
            if at_rc in heads:
                break
        links.append({"order": order[head], "length": length, "cells": cells,
                      "outlet_area": acc[at_rc[0]][at_rc[1]] * cell * cell})
    by = {}
    for l in links:
        n, total = by.get(l["order"], (0, 0.0))
        by[l["order"]] = (n + 1, total + l["length"])
    return {
        "cells": len(chan), "highest": max(order.values(), default=0),
        "links": links, "by_order": sorted(by.items()),
        "length": sum(l["length"] for l in links),
        "max_acc": max((acc[r][c] for r in range(h) for c in range(w)
                        if grid[r][c] is not None), default=0.0),
    }


def case(rows, grid, cell, threshold, source, no_data=None, detail=False):
    n = network(grid, cell, threshold)
    inp = {"elevations": rows_of(grid, no_data), "cell_size": f"{cell} m",
           "threshold_cells": threshold}
    if no_data is not None:
        inp["no_data"] = no_data
    ex = {"ok": True, "result.highest_order": n["highest"],
          "result.links_found": len(n["links"]),
          "result.channel_cells": n["cells"],
          "result.network_length.value": n["length"],
          "result.max_accumulation": n["max_acc"]}
    if detail:
        for i, (order, (count, length)) in enumerate(n["by_order"]):
            ex[f"result.by_order.{i}.order"] = order
            ex[f"result.by_order.{i}.links"] = count
            ex[f"result.by_order.{i}.length.value"] = length
        for i, l in enumerate(n["links"]):
            ex[f"result.links.{i}.order"] = l["order"]
            ex[f"result.links.{i}.cells"] = l["cells"]
            ex[f"result.links.{i}.outlet_area.value"] = l["outlet_area"]
    if n["cells"] == 0:
        ex["meta.warnings.*.code"] = "EMPTY_NETWORK"
    tol = {k: {"abs": 1e-9 if k.endswith(".value") else 0}
           for k, v in ex.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    rows.append({"input": inp, "expect": ex, "source": source,
                 "sourceVersion": "Python math; Strahler 1957; D8 accumulation walked downhill",
                 "tolerance": tol})


def main():
    rows = []
    SPEC = "add-raster-hydrology raster/hydrology scenario"
    PAPERS = "Strahler (1957) ordering over a D8 network, reimplemented in Python"

    # The Order rule scenario: two headwaters joining into one stream.
    fork = [[60.0, 48.0, 60.0, 60.0, 60.0, 48.0, 60.0],
            [60.0, 60.0, 47.0, 60.0, 47.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 46.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 45.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 44.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 43.0, 60.0, 60.0, 60.0],
            [60.0, 60.0, 60.0, 42.0, 60.0, 60.0, 60.0]]
    for thr in (2, 3, 4, 6):
        case(rows, fork, 30.0, thr, SPEC + f": the Order rule, threshold {thr} cells", detail=True)
    # The Threshold too high scenario.
    case(rows, fork, 30.0, 10_000, SPEC + ": a threshold no cell reaches, with the window's maximum given")

    # A valley gathering from both sides: one stem, many short tributaries.
    valley = [[100.0 - r + abs(c - 5) * 3.0 for c in range(11)] for r in range(11)]
    for thr in (5, 15, 40):
        case(rows, valley, 20.0, thr, PAPERS + f": a valley draining south, threshold {thr}", detail=(thr == 15))

    # A cone: every cell drains outward, so the network is many short links.
    cone = [[200.0 - math.hypot(r - 7.5, c - 7.5) * 10.0 for c in range(16)] for r in range(16)]
    for thr in (3, 10):
        case(rows, cone, 10.0, thr, PAPERS + f": an outward draining cone, threshold {thr}")

    # A no-data hole.
    hole = [[100.0 - r + abs(c - 3) * 2.0 for c in range(7)] for r in range(7)]
    hole[3][5] = None
    case(rows, hole, 15.0, 4, PAPERS + ": a valley with a no-data hole beside it", no_data=-9999)

    # Seeded random terrain at three sizes and two thresholds each.
    rng = random.Random(23)
    for n in (9, 12, 15):
        grid = [[round(140.0 - r * 1.5 + 5.0 * math.cos(c / 2.0) + rng.uniform(-1.0, 1.0), 3)
                 for c in range(n)] for r in range(n)]
        for thr in (4, 12):
            case(rows, grid, 25.0, thr, f"{PAPERS}: {n} by {n} of seeded random terrain, threshold {thr}")

    # Refusals: no threshold, both thresholds, one corner only.
    base = {"elevations": rows_of(fork), "cell_size": "30.0 m"}
    rows.append({"input": base, "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": the threshold is required", "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {**base, "threshold_cells": 3, "threshold_area": "2700 m2"},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": the threshold comes one way or the other, not both",
                 "sourceVersion": "2026", "tolerance": {}})
    rows.append({"input": {**base, "threshold_cells": 3, "north_west_lat": "44.12 deg"},
                 "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": SPEC + ": a corner needs both its latitude and its longitude",
                 "sourceVersion": "2026", "tolerance": {}})

    for i, r in enumerate(rows, 1):
        print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
