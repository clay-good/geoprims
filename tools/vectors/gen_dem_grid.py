#!/usr/bin/env python3
"""Golden vectors for raster.terrain.dem-stats, highest-point, and
dem-difference-volume, computed with NumPy (not the core). Every dem-stats grid
is also written as an ASCII grid and read back by GDAL's gdalinfo -stats
(3.13.3, from Homebrew); the script stops if GDAL's minimum, maximum, mean, or
population standard deviation disagree beyond its float32 storage.

Usage: python3 tools/vectors/gen_dem_grid.py [out_dir] [scratch_dir]
"""
import json
import random
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

SRC = "NumPy on the same grid (tools/vectors/gen_dem_grid.py)"
SRC_GDAL = "NumPy, with gdalinfo 3.13.3 -stats for the minimum, maximum, mean, and standard deviation (tools/vectors/gen_dem_grid.py)"
VER = "2026-10-14"
ND = -9999.0


def rows(z):
    return [{"row": ", ".join(repr(float(v)) for v in r)} for r in z]


def gdal_stats(z, tmp):
    asc, out = Path(tmp) / "g.asc", Path(tmp) / "g.tif"
    body = "\n".join(" ".join(repr(float(v)) for v in r) for r in z)
    asc.write_text(f"ncols {len(z[0])}\nnrows {len(z)}\nxllcorner 0\nyllcorner 0\ncellsize 1\nNODATA_value {ND!r}\n{body}\n")
    subprocess.run(["gdal_translate", "-q", "-ot", "Float64", str(asc), str(out)], check=True)
    text = subprocess.run(["gdalinfo", "-stats", str(out)], capture_output=True, text=True, check=True).stdout
    get = lambda k: float(re.search(rf"STATISTICS_{k}=(\S+)", text).group(1))
    for aux in Path(tmp).glob("g.tif.aux.xml"):
        aux.unlink()
    return get("MINIMUM"), get("MAXIMUM"), get("MEAN"), get("STDDEV")


def surface(rng, nr, nc, base, relief):
    a, b, c = rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(0, 1)
    return [[round(base + relief * (a * i + b * j + c * np.sin(i + 2 * j)) / max(nr, nc) + rng.uniform(-0.2, 0.2), 2)
             for j in range(nc)] for i in range(nr)]


def vec(n, inp, exp, src, tol=1e-9):
    e = {"ok": True, **exp}
    t = {k: {"abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{n:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def err(n, inp, field):
    return {"id": f"v{n:03d}", "input": inp, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": field},
            "source": "add-spatial-indexing-and-raster scenarios", "sourceVersion": "2026", "tolerance": {}}


def main():
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    tmp = sys.argv[2] if len(sys.argv) > 2 else tempfile.mkdtemp()
    rng = random.Random(1014)
    stats, high, diff = [], [], []
    grids = [
        [[212.4, 213.1, 214.0, 214.8, 215.2], [211.9, 212.6, 213.7, 214.5, 215.0], [211.2, 212.0, 213.1, 214.1, 214.6], [210.8, 211.5, 212.4, 213.6, 214.2]],
        [[48.2, 48.9, 49.1, 48.7], [48.8, 50.3, 50.6, 49.0], [48.5, 50.1, 50.6, 48.9], [48.0, 48.6, 48.8, 48.4]],
        [[5.0, 5.0], [5.0, 5.0]],
        [[100.0, ND, 102.0], [ND, 104.0, 101.0]],
    ]
    while len(grids) < 24:
        nr, nc = rng.randint(2, 12), rng.randint(2, 12)
        z = surface(rng, nr, nc, rng.uniform(-50, 3000), rng.choice([1, 10, 100, 400]))
        if rng.random() < 0.3:
            for _ in range(rng.randint(1, nr * nc // 3)):
                z[rng.randrange(nr)][rng.randrange(nc)] = ND
        if rng.random() < 0.25:  # a flat top: two cells share the maximum
            m = max(v for r in z for v in r if v != ND)
            z[rng.randrange(nr)][rng.randrange(nc)] = m
        grids.append(z)
    for k, z in enumerate(grids):
        a = np.array(z, dtype=float)
        v = a[a != ND]
        if v.size == 0:
            continue
        lo, hi, mean, sd = gdal_stats(z, tmp)
        # The ASCII grid driver stores float32, so allow its rounding.
        f32 = 2.0 ** -23 * max(1.0, float(np.abs(v).max()))
        assert abs(lo - v.min()) <= f32 and abs(hi - v.max()) <= f32, (z, lo, hi)
        assert abs(mean - v.mean()) <= f32 and abs(sd - v.std()) <= 2 * f32, (mean, v.mean(), sd, v.std())
        inp = {"elevations": rows(z)}
        if (a == ND).any():
            inp["no_data"] = ND
        stats.append(vec(len(stats) + 1, inp, {"result.mean.value": float(v.mean()), "result.lowest.value": float(v.min()),
                                               "result.highest.value": float(v.max()), "result.relief.value": float(v.max() - v.min()),
                                               "result.std_dev.value": float(v.std()), "result.median.value": float(np.median(v)),
                                               "result.cells": float(v.size), "result.no_data_cells": float(a.size - v.size)}, SRC_GDAL))
        masked = np.where(a == ND, -np.inf, a)
        r, c = np.unravel_index(np.argmax(masked), a.shape)
        cell = rng.choice([None, 1.0, 2.0, 5.0, 30.0]) if k != 1 else 2.0
        hin = dict(inp)
        exp = {"result.highest.value": float(a[r, c]), "result.row": float(r + 1), "result.column": float(c + 1),
               "result.ties": float((a == a[r, c]).sum())}
        if cell:
            hin["cell_size"] = f"{cell:g} m"
            exp |= {"result.east.value": float(c * cell), "result.south.value": float(r * cell)}
        high.append(vec(len(high) + 1, hin, exp, SRC))
    # Difference volumes: the example stockpile, then surveys of the same ground.
    pairs = [([[10.0] * 4] * 3, [[10.0, 10.4, 10.6, 10.1], [10.2, 11.3, 11.8, 10.3], [9.9, 10.2, 10.5, 9.8]], 1.0, 0.05)]
    while len(pairs) < 24:
        nr, nc = rng.randint(2, 10), rng.randint(2, 10)
        before = surface(rng, nr, nc, rng.uniform(0, 500), rng.choice([2, 20, 100]))
        after = [[round(v + rng.choice([0, 0, rng.uniform(-0.03, 0.03), rng.uniform(-2, 2)]), 3) for v in r] for r in before]
        if rng.random() < 0.3:
            after[rng.randrange(nr)][rng.randrange(nc)] = ND
        pairs.append((before, after, rng.choice([0.5, 1.0, 2.0, 5.0, 10.0]), rng.choice([None, 0.0, 0.05, 0.1])))
    for before, after, cell, floor in pairs:
        b, a = np.array(before, float), np.array(after, float)
        ok = (b != ND) & (a != ND)
        d = np.where(ok, a - b, 0.0)
        d = np.where(np.abs(d) < (floor or 0.0), 0.0, d)
        area = cell * cell
        inp = {"before": rows(before), "after": rows(after), "cell_size": f"{cell:g} m"}
        if floor is not None:
            inp["min_change"] = f"{floor:g} m"
        if (a == ND).any() or (b == ND).any():
            inp["no_data"] = ND
        diff.append(vec(len(diff) + 1, inp, {"result.net_volume.value": float(d.sum() * area), "result.gained_volume.value": float(d[d > 0].sum() * area),
                                             "result.lost_volume.value": float(-d[d < 0].sum() * area), "result.changed_area.value": float((d != 0).sum() * area),
                                             "result.mean_change.value": float(d.sum() / ok.sum()), "result.cells": float(ok.sum()),
                                             "result.skipped_cells": float((~ok).sum())}, SRC))
    two = [{"row": "1, 2"}, {"row": "3, 4"}]
    stats.append(err(len(stats) + 1, {"elevations": [{"row": f"{ND}, {ND}"}, {"row": f"{ND}, {ND}"}], "no_data": ND}, "/elevations"))
    high.append(err(len(high) + 1, {"elevations": [{"row": "1, 2"}, {"row": "3"}]}, "/elevations/1/row"))
    diff.append(err(len(diff) + 1, {"before": two, "after": two + [{"row": "5, 6"}], "cell_size": "1 m"}, "/after"))
    diff.append(err(len(diff) + 1, {"before": two, "after": two}, "/cell_size"))
    for name, vs in [("dem-stats", stats), ("highest-point", high), ("dem-difference-volume", diff)]:
        (out_dir / f"raster.terrain.{name}.jsonl").write_text("".join(json.dumps(v, separators=(",", ":")) + "\n" for v in vs))
        print(name, len(vs))


if __name__ == "__main__":
    main()
