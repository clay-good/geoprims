#!/usr/bin/env python3
"""Terrain derivatives against GDAL's gdaldem (raster/terrain-analysis,
"GDAL agreement" scenario; promotion of raster.terrain.slope-aspect and
raster.terrain.ruggedness).

Writes three projected DEMs as ASCII grids (5, 10, and 30 m cells; smooth
hills with noise, one with a flat patch), runs gdaldem slope, aspect,
hillshade (Horn, sun 315 deg at 45 deg), TRI by both Riley's and Wilson's
algorithms, TPI, and roughness on each, and records, for every interior cell,
its 3 x 3 window and gdaldem's values there. gdaldem holds elevations as
32-bit floats, so slope agrees to about 1e-4 degrees and, from half a degree
of slope up, aspect within the spec's 0.01 degrees; on nearly flat ground
aspect is ill-conditioned and gdaldem's own rounding sets the bound.

Writes core/crates/gp-raster/tests/data/terrain_gdal.json and appends
vectors to both tools' files (existing lines left byte for byte).
Requires gdaldem (GDAL 3.x)."""
import json
import math
import random
import subprocess
import tempfile
from pathlib import Path

FIX = Path("core/crates/gp-raster/tests/data/terrain_gdal.json")


def version():
    return subprocess.run(["gdaldem", "--version"], capture_output=True, text=True).stdout.strip() or "GDAL"


def dem(rng, n, flat):
    bumps = [(rng.uniform(0, n), rng.uniform(0, n), rng.uniform(-30, 40), rng.uniform(3, 9)) for _ in range(4)]
    z = []
    for i in range(n):
        row = []
        for j in range(n):
            if flat and 3 <= i <= 8 and 3 <= j <= 8:
                row.append(250.0)
                continue
            v = 250 + sum(h * math.exp(-((i - a) ** 2 + (j - b) ** 2) / (2 * s * s)) for a, b, h, s in bumps)
            row.append(round(v + rng.uniform(-0.3, 0.3), 2))
        z.append(row)
    return z


def write(path, z, cell):
    n = len(z)
    head = f"ncols {n}\nnrows {n}\nxllcorner 0\nyllcorner 0\ncellsize {cell}\n"
    path.write_text(head + "".join(" ".join(repr(v) for v in r) + "\n" for r in z))


def read(path):
    lines = path.read_text().splitlines()
    nodata = float([l for l in lines if l.lower().startswith("nodata")][0].split()[1])
    body = [l for l in lines if l and (l[0].isdigit() or l[0] in "-.")]
    return [[None if float(x) == nodata else float(x) for x in l.split()] for l in body], nodata


def run(d, src, what, extra):
    out = d / f"{what}_{'_'.join(extra) or 'x'}.asc"
    subprocess.run(["gdaldem", what, str(src), str(out), "-of", "AAIGrid", "-co", "SIGNIFICANT_DIGITS=17", "-q", *extra], check=True)
    return read(out)[0]


def main():
    rng = random.Random(21)
    windows = []
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        for k, (cell, flat) in enumerate([(10.0, False), (30.0, True), (5.0, False)]):
            z = dem(rng, 24, flat)
            src = d / f"dem{k}.asc"
            write(src, z, cell)
            grids = {
                "slope": run(d, src, "slope", []),
                "aspect": run(d, src, "aspect", []),
                "hillshade": run(d, src, "hillshade", []),
                "tri": run(d, src, "TRI", ["-alg", "Riley"]),
                "tri_mean": run(d, src, "TRI", ["-alg", "Wilson"]),
                "tpi": run(d, src, "TPI", []),
                "roughness": run(d, src, "roughness", []),
            }
            n = len(z)
            for i in range(1, n - 1):
                for j in range(1, n - 1):
                    rows = [", ".join(repr(z[i + di][j + dj]) for dj in (-1, 0, 1)) for di in (-1, 0, 1)]
                    windows.append({"cell_size": cell, "rows": rows, **{k2: g[i][j] for k2, g in grids.items()}})
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": version(), "windows": windows}, separators=(",", ":")) + "\n")
    print(len(windows), "windows,", sum(w["aspect"] is None for w in windows), "flat")

    src = f"gdaldem ({version()}) on a projected DEM, the cell's 3 x 3 window (tools/vectors/gen_terrain_gdal.py)"
    # Away from near-flat ground, where gdaldem's 32-bit aspect wanders.
    picks = [w for w in windows if w["aspect"] is not None and w["slope"] >= 1.0][::97][:13] + [w for w in windows if w["aspect"] is None][:1]
    for tool, fields in [("raster.terrain.slope-aspect", ["slope", "aspect", "hillshade"]), ("raster.terrain.ruggedness", ["tri", "tri_mean", "tpi", "roughness"])]:
        path = Path(f"core/vectors/{tool}.jsonl")
        lines = path.read_text().splitlines()
        kept = [l for l in lines if "gen_terrain_gdal.py" not in l]
        n, new = len(kept), []
        for w in picks:
            exp, tol = {"ok": True}, {}
            for f in fields:
                if w[f] is None:
                    continue
                key = f"result.{f}" if f == "hillshade" else f"result.{f}.value"
                exp[key] = round(w[f]) if f == "hillshade" else w[f]
                # gdaldem works in 32-bit floats: agreement is to about 1e-4
                # degrees and a tenth of a millimeter, inside the spec's 0.01.
                tol[key] = {"rel": 0, "abs": 1.0} if f == "hillshade" else {"rel": 0, "abs": 1e-2 if f == "aspect" else 1e-3}
            if tool.endswith("slope-aspect") and w["aspect"] is None:
                exp["result.aspect_text"] = "flat"
            new.append({"id": f"v{n + len(new) + 1:03d}", "input": {"elevations": [{"row": r} for r in w["rows"]], "cell_size": f"{w['cell_size']} m"},
                        "expect": exp, "source": src, "sourceVersion": version(), "tolerance": tol})
        path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
        print(tool, n, "->", n + len(new))


if __name__ == "__main__":
    main()
