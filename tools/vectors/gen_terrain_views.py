#!/usr/bin/env python3
"""Golden vectors for the focused terrain operations raster.terrain.slope,
aspect, hillshade, roughness-tri, and tpi.

Each value is Horn's formula or the index definition transcribed here in
64-bit Python, independently of the Rust code, and every window is also run
through GDAL's gdaldem (3.13.3, from Homebrew) as a second check: the script
stops if the two disagree beyond gdaldem's 32-bit arithmetic. Usage:
  python3 tools/vectors/gen_terrain_views.py [out_dir] [scratch_dir]
"""
import json
import math
import random
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = "Horn's formulas and the TRI and TPI definitions in Python, each window checked against gdaldem 3.13.3 (tools/vectors/gen_terrain_views.py)"
VER = "2026-10-14"


def horn(z, cell, zf=1.0):
    a, b, c, d, e, f, g, h, i = z
    gx = zf * ((c + 2 * f + i) - (a + 2 * d + g)) / (8 * cell)
    gy = zf * ((g + 2 * h + i) - (a + 2 * b + c)) / (8 * cell)
    return gx, gy


def slope(z, cell, zf=1.0):
    gx, gy = horn(z, cell, zf)
    r = math.hypot(gx, gy)
    return math.degrees(math.atan(r)), 100 * r


def aspect(z, cell):
    gx, gy = horn(z, cell)
    if gx == 0 and gy == 0:
        return None
    return (90 - math.degrees(math.atan2(gy, -gx))) % 360


def facing(a):
    if a is None:
        return "flat"
    return ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"][int(((a + 22.5) % 360) / 45)]


def shade(z, cell, zf=1.0, az=315.0, alt=45.0):
    gx, gy = horn(z, cell, zf)
    s = math.atan(math.hypot(gx, gy))
    asp = 0.0 if gx == 0 and gy == 0 else math.atan2(gy, -gx)
    zen, sun = math.radians(90 - alt), math.radians(90 - az)
    cang = math.cos(zen) * math.cos(s) + math.sin(zen) * math.sin(s) * math.cos(sun - asp)
    # gdaldem rounds half up into its byte.
    return 1 if cang <= 0 else math.floor(1 + 254 * cang + 0.5)


def tri(z):
    n = [v for k, v in enumerate(z) if k != 4]
    return math.sqrt(sum((v - z[4]) ** 2 for v in n)), sum(abs(v - z[4]) for v in n) / 8


def tpi(z):
    return z[4] - (sum(z) - z[4]) / 8


def position(t):
    return ("above its neighbours: a ridge, a summit, or a spur" if t > 0 else
            "below its neighbours: a valley, a pit, or a hollow" if t < 0 else "level with its neighbours")


class Gdal:
    def __init__(self, tmp):
        self.tmp = Path(tmp)

    def run(self, mode, z, cell, *args):
        asc, tif = self.tmp / "w.asc", self.tmp / f"{mode}.tif"
        rows = "\n".join(" ".join(repr(v) for v in z[k:k + 3]) for k in (0, 3, 6))
        asc.write_text(f"ncols 3\nnrows 3\nxllcorner 0\nyllcorner 0\ncellsize {cell!r}\n{rows}\n")
        subprocess.run(["gdaldem", mode, str(asc), str(tif), "-q", *args], check=True)
        out = subprocess.run(["gdallocationinfo", "-valonly", str(tif), "1", "1"], capture_output=True, text=True, check=True).stdout.strip()
        return float(out) if out else None


def windows():
    rng = random.Random(14102026)
    fixed = [
        ([250.0, 252.5, 255.1, 248.2, 251.0, 254.9, 246.0, 249.3, 253.3], 10.0),
        ([500.0, 498.0, 495.0, 503.0, 500.0, 497.0, 507.0, 504.0, 500.0], 10.0),
        ([10.0, 12.0, 11.0, 9.0, 13.0, 14.0, 8.5, 11.5, 12.5], 5.0),
        ([1210.4, 1225.9, 1218.2, 1204.7, 1220.0, 1231.6, 1199.3, 1212.8, 1226.1], 30.0),
        ([842.0, 845.5, 841.2, 846.1, 851.3, 847.0, 840.4, 844.8, 839.9], 30.0),
        ([75.0, 75.0, 75.0, 75.0, 75.0, 75.0, 75.0, 75.0, 75.0], 30.0),
        ([12.0, 14.0, 16.0, 10.0, 12.0, 14.0, 8.0, 10.0, 12.0], 1.0),
    ]
    out = list(fixed)
    while len(out) < 24:
        base, cell = rng.uniform(0, 3000), rng.choice([1.0, 2.0, 5.0, 10.0, 30.0, 90.0])
        relief = cell * rng.choice([0.01, 0.1, 0.4, 1.0, 2.5])
        tilt = (rng.uniform(-1, 1), rng.uniform(-1, 1))
        z = [round(base + relief * (tilt[0] * (c - 1) - tilt[1] * (r - 1)) + rng.uniform(-0.3, 0.3) * relief, 2)
             for r in range(3) for c in range(3)]
        out.append((z, cell))
    return out


def vector(n, inp, expect, tol=1e-9):
    e = {"ok": True, **expect}
    t = {k: {"abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{n:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t}


def main():
    out_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    tmp = sys.argv[2] if len(sys.argv) > 2 else tempfile.mkdtemp()
    gd = Gdal(tmp)
    rng = random.Random(9)
    sets = {k: [] for k in ("slope", "aspect", "hillshade", "roughness-tri", "tpi")}
    for k, (z, cell) in enumerate(windows()):
        inp = {"elevations": [{"row": ", ".join(repr(v) for v in z[k:k + 3])} for k in (0, 3, 6)], "cell_size": f"{cell:g} m"}
        s, pct = slope(z, cell)
        # gdaldem holds elevations as 32-bit floats: rounding moves each Horn
        # gradient by up to dg, which on nearly flat ground is a large share.
        dg = 4 * max(map(abs, z)) * 2.0 ** -23 * math.sqrt(2) / (8 * cell)
        assert abs(s - gd.run("slope", z, cell)) < max(1e-3, 2 * math.degrees(dg)), (z, cell)
        sets["slope"].append((inp, {"result.slope.value": s, "result.slope_percent": pct}))
        a = aspect(z, cell)
        g = gd.run("aspect", z, cell)
        if a is None or g is None or g == -9999:
            assert a is None and (g is None or g == -9999), (z, a, g)
            sets["aspect"].append((inp, {"result.aspect_text": "flat"}))
        else:
            d = abs(a - g) % 360
            # Aspect moves by about dg / |gradient| radians under that rounding.
            bound = max(0.01, 2 * math.degrees(dg / math.tan(math.radians(s))))
            assert min(d, 360 - d) < bound, (z, a, g)
            sets["aspect"].append((inp, {"result.aspect_text": facing(a), "result.aspect.value": a}))
        # The third window is the hillshade page's example, lit from the west.
        az, alt = (270.0, 30.0) if k == 2 else rng.choice([(315.0, 45.0), (270.0, 30.0), (135.0, 60.0), (0.0, 20.0)])
        sh = shade(z, cell, 1.0, az, alt)
        assert sh == gd.run("hillshade", z, cell, "-az", repr(az), "-alt", repr(alt)), (z, az, alt)
        hin = dict(inp)
        if (az, alt) != (315.0, 45.0):
            hin |= {"sun_azimuth": f"{az:g} deg", "sun_altitude": f"{alt:g} deg"}
        sets["hillshade"].append((hin, {"result.hillshade": sh}))
        r, m = tri(z)
        dz = 8 * max(map(abs, z)) * 2.0 ** -23
        assert abs(r - gd.run("TRI", z, cell, "-alg", "Riley")) < max(1e-3, dz) and abs(m - gd.run("TRI", z, cell, "-alg", "Wilson")) < max(1e-3, dz)
        sets["roughness-tri"].append((inp, {"result.tri.value": r, "result.tri_mean.value": m}))
        t = tpi(z)
        assert abs(t - gd.run("TPI", z, cell)) < max(1e-3, dz)
        sets["tpi"].append((inp, {"result.tpi.value": t, "result.position": position(t)}))
    bad = {"elevations": [{"row": "1, 2, 3"}, {"row": "4, 5, 6"}], "cell_size": "10 m"}
    for name, rows in sets.items():
        vs = [vector(n, i, e) for n, (i, e) in enumerate(rows, 1)]
        vs.append({"id": f"v{len(vs) + 1:03d}", "input": bad, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/elevations"},
                   "source": "add-spatial-indexing-and-raster scenarios", "sourceVersion": "2026", "tolerance": {}})
        (out_dir / f"raster.terrain.{name}.jsonl").write_text("".join(json.dumps(v, separators=(",", ":")) + "\n" for v in vs))
        print(name, len(vs))


if __name__ == "__main__":
    main()
