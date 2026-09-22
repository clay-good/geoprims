#!/usr/bin/env python3
"""Golden vectors for raster.index.*.

Each index is transcribed here straight from the formula in its own paper,
independently of the Rust implementation, and evaluated in Python. The values
are therefore a second reading of the published definition rather than a copy
of what the core returns.

Formulas (see each tool's citation for the paper):
  NDVI   (NIR - Red) / (NIR + Red)                        Rouse and others 1974
  NDWI   (Green - NIR) / (Green + NIR)                    McFeeters 1996
  NDWI   (NIR - SWIR1) / (NIR + SWIR1)                    Gao 1996
  MNDWI  (Green - SWIR1) / (Green + SWIR1)                Xu 2006
  NDBI   (SWIR1 - NIR) / (SWIR1 + NIR)                    Zha and others 2003
  NBR    (NIR - SWIR2) / (NIR + SWIR2)                    Key and Benson 2006
  EVI    2.5 (NIR - Red) / (NIR + 6 Red - 7.5 Blue + 1)   Huete and others 2002
  EVI2   2.5 (NIR - Red) / (NIR + 2.4 Red + 1)            Jiang and others 2008
  SAVI   (1 + L)(NIR - Red) / (NIR + Red + L)             Huete 1988
  dNBR   NBR(pre) - NBR(post)                             Key and Benson 2006
"""
import json
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core" / "vectors"
SRC = "Index formulas transcribed from their papers in Python (tools/vectors/gen_raster.py)"
VER = "2026-09-22"


def vec(i, inp, expect, tol=1e-12):
    e = dict(expect)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t}


def write(name, rows):
    path = OUT / f"raster.index.{name}.jsonl"
    path.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
    print(f"{path.name}: {len(rows)}")


# Reflectance pairs spread over vegetation, water, soil, and burned ground.
PAIRS = [(0.45, 0.08), (0.38, 0.22), (0.12, 0.04), (0.31, 0.26), (0.33, 0.08),
         (0.05, 0.4), (0.9, 0.05), (0.02, 0.02), (0.6, 0.15), (0.21, 0.19)]


def normalized(rows_for, key, a_name, b_name):
    rows = []
    for i, (a, b) in enumerate(PAIRS, 1):
        rows.append(vec(i, {a_name: a, b_name: b}, {f"result.{key}": (a - b) / (a + b)}))
    write(rows_for, rows)


normalized("ndvi", "ndvi", "nir", "red")
normalized("ndwi-mcfeeters", "ndwi", "green", "nir")
normalized("ndwi-gao", "ndwi", "nir", "swir1")
normalized("mndwi", "mndwi", "green", "swir1")
normalized("ndbi", "ndbi", "swir1", "nir")
normalized("nbr", "nbr", "nir", "swir2")

rows = []
for i, (nir, red) in enumerate(PAIRS, 1):
    blue = round(red * 0.5, 6)
    rows.append(vec(i, {"nir": nir, "red": red, "blue": blue},
                    {"result.evi": 2.5 * (nir - red) / (nir + 6 * red - 7.5 * blue + 1)}))
write("evi", rows)

rows = [vec(i, {"nir": nir, "red": red}, {"result.evi2": 2.5 * (nir - red) / (nir + 2.4 * red + 1)})
        for i, (nir, red) in enumerate(PAIRS, 1)]
write("evi2", rows)

rows = []
i = 0
for nir, red in PAIRS:
    for l in (0.5, 0.0, 1.0):
        i += 1
        inp = {"nir": nir, "red": red} if l == 0.5 else {"nir": nir, "red": red, "soil_factor": l}
        rows.append(vec(i, inp, {"result.savi": (1 + l) * (nir - red) / (nir + red + l)}))
write("savi", rows)

# dNBR over pre- and post-fire pairs, with the Key and Benson class each falls in.
CLASSES = [(-0.25, "high post-fire regrowth"), (-0.1, "low post-fire regrowth"), (0.1, "unburned"),
           (0.27, "low severity"), (0.44, "moderate-low severity"), (0.66, "moderate-high severity")]
rows = []
for i, (pre, post) in enumerate([(0.61, 0.13), (0.7, 0.05), (0.5, 0.45), (0.4, 0.2), (0.55, 0.2),
                                 (0.2, 0.5), (0.3, 0.45), (0.8, -0.1), (0.1, 0.1), (0.45, 0.3)], 1):
    d = pre - post
    name = next((n for edge, n in CLASSES if d < edge), "high severity")
    rows.append(vec(i, {"nbr_pre": pre, "nbr_post": post}, {"result.dnbr": d, "result.severity": name}))
write("dnbr", rows)

# Sensor scaling, from each product's published relation:
#   Sentinel-2 L2A: (DN + BOA_ADD_OFFSET) / QUANTIFICATION_VALUE, offset -1000
#     from processing baseline 04.00 (SentiWiki, Sentinel-2 products)
#   Landsat Collection 2 Level-2: DN * 0.0000275 - 0.2 (USGS scale factor FAQ)
rows = []
i = 0
for dn in (1450, 2000, 1001, 9000, 1200, 3500):
    i += 1
    rows.append(vec(i, {"dn": dn, "sensor": "sentinel-2-l2a"}, {"result.reflectance": (dn - 1000) / 10000}))
    i += 1
    rows.append(vec(i, {"dn": dn, "sensor": "sentinel-2-l2a", "baseline": "before-04.00"},
                    {"result.reflectance": dn / 10000}))
for dn in (18639, 7273, 43636, 10000, 30000):
    i += 1
    rows.append(vec(i, {"dn": dn, "sensor": "landsat-c2-l2"}, {"result.reflectance": dn * 0.0000275 - 0.2}))
path = OUT / "raster.scale.reflectance.jsonl"
path.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print(f"{path.name}: {len(rows)}")

# Band math: the same formulas written as expressions, so the parser and the
# evaluator are checked against the arithmetic rather than against themselves.
rows = []
for i, (expr, bands, want) in enumerate([
    ("(nir - red) / (nir + red)", {"nir": 0.45, "red": 0.08}, (0.45 - 0.08) / (0.45 + 0.08)),
    ("2.5 * (nir - red) / (nir + 2.4 * red + 1)", {"nir": 0.45, "red": 0.08},
     2.5 * (0.45 - 0.08) / (0.45 + 2.4 * 0.08 + 1)),
    ("a + b * c", {"a": 2, "b": 3, "c": 4}, 2 + 3 * 4),
    ("(a + b) * c", {"a": 2, "b": 3, "c": 4}, (2 + 3) * 4),
    ("a < b ? c : a", {"a": 2, "b": 3, "c": 4}, 4),
    ("a > b ? c : a", {"a": 2, "b": 3, "c": 4}, 2),
    ("clamp(x, 0, 1)", {"x": 1.4}, 1.0),
    ("clamp(x, 0, 1)", {"x": -0.3}, 0.0),
    ("min(a, b) + max(a, b)", {"a": 2, "b": 3}, 5),
    ("sqrt(a) * exp(0) - abs(0 - b)", {"a": 9, "b": 2}, 3 - 2),
    ("nir + red > 0 ? (nir - red) / (nir + red) : -999", {"nir": 0.0, "red": 0.0}, -999),
    ("-a + b * -c", {"a": 2, "b": 3, "c": 4}, -2 + 3 * -4),
], 1):
    inp = {"expression": expr, "bands": [{"name": n, "value": v} for n, v in bands.items()]}
    rows.append(vec(i, inp, {"result.value": float(want)}))
path = OUT / "raster.index.band-math.jsonl"
path.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print(f"{path.name}: {len(rows)}")
