#!/usr/bin/env python3
"""Spectral indices against spyndex (promotion of nine raster.index tools).

spyndex is the Python front end of Awesome Spectral Indices (Montero and
others, Scientific Data 10, 197, 2023), a curated catalog of spectral index
formulas, each tied to its original publication, written independently of
geoprims. Each tool's value is compared with spyndex.computeIndex on the
same reflectances, with the catalog's constants (EVI: g 2.5, C1 6, C2 7.5,
L 1; EVI2: g 2.5, L 1; SAVI: L as given, 0.5 by default).

The catalog's names differ in two places: its NDWI is McFeeters' (green and
NIR) and Gao's NDWI (NIR and SWIR1) is its NDMI.

Writes core/crates/gp-raster/tests/data/indices_spyndex.json (400 cases per
index) and appends 10 vectors to each tool's file (existing lines left byte
for byte). Requires spyndex."""
import json
import random
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import spyndex  # noqa: E402

# tool, catalog index, {tool input: catalog band}, output field
INDICES = [
    ("raster.index.ndvi", "NDVI", {"nir": "N", "red": "R"}, "ndvi"),
    ("raster.index.evi", "EVI", {"nir": "N", "red": "R", "blue": "B"}, "evi"),
    ("raster.index.evi2", "EVI2", {"nir": "N", "red": "R"}, "evi2"),
    ("raster.index.savi", "SAVI", {"nir": "N", "red": "R"}, "savi"),
    ("raster.index.ndwi-mcfeeters", "NDWI", {"green": "G", "nir": "N"}, "ndwi"),
    ("raster.index.ndwi-gao", "NDMI", {"nir": "N", "swir1": "S1"}, "ndwi"),
    ("raster.index.mndwi", "MNDWI", {"green": "G", "swir1": "S1"}, "mndwi"),
    ("raster.index.ndbi", "NDBI", {"swir1": "S1", "nir": "N"}, "ndbi"),
    ("raster.index.nbr", "NBR", {"nir": "N", "swir2": "S2"}, "nbr"),
]
FIX = Path("core/crates/gp-raster/tests/data/indices_spyndex.json")
CONST = {"g": 2.5, "C1": 6.0, "C2": 7.5}


def value(index, bands, soil=None):
    params = dict(bands)
    params.update(CONST)
    params["L"] = soil if index == "SAVI" else 1.0
    return float(spyndex.computeIndex(index, params=params))


def main():
    rng = random.Random(9)
    fixture = {}
    for tool, index, band_map, out in INDICES:
        rows = []
        for _ in range(400):
            inp = {name: round(rng.uniform(0.01, 0.15 if name == "blue" else 0.6), 4) for name in band_map}
            soil = round(rng.uniform(0.0, 1.0), 3) if index == "SAVI" else None
            if soil is not None:
                inp["soil_factor"] = soil
            v = value(index, {band_map[k]: inp[k] for k in band_map}, soil)
            rows.append({"input": inp, "value": v})
        fixture[tool] = {"index": index, "output": out, "cases": rows}

        path = Path(f"core/vectors/{tool}.jsonl")
        lines = path.read_text().splitlines()
        kept = [l for l in lines if "gen_indices_spyndex.py" not in l]
        n, new = len(kept), []
        for r in rows[:10]:
            new.append({
                "id": f"v{n + len(new) + 1:03d}", "input": r["input"],
                "expect": {f"result.{out}": r["value"], "ok": True},
                "source": f"spyndex {spyndex.__version__} computeIndex('{index}'), Awesome Spectral Indices (tools/vectors/gen_indices_spyndex.py)",
                "sourceVersion": f"spyndex {spyndex.__version__}",
                "tolerance": {f"result.{out}": {"rel": 1e-12, "abs": 1e-12}},
            })
        path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
        print(tool, n, "->", n + len(new))
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": f"spyndex {spyndex.__version__}", "tools": fixture}, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
