#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.lv95-forward and lv95-inverse
(add-international-grids, geodesy/national-grids).

The oracle is PROJ 9.9.0 through cs2cs applying EPSG:2056 (CH1903+ / LV95)
from CH1903+ geographic coordinates, so the projection parameters come from
the EPSG dataset rather than from this repository. swisstopo's own worked
example for the station Rigi is the first vector, from their December 2016
"Formulas and constants" document.

    gen_lv95.py OUT_INVERSE   # prints the forward vectors, writes the inverse
"""
import json
import subprocess
import sys

CH1903 = "+proj=longlat +ellps=bessel +no_defs"
LV95 = "EPSG:2056"


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10", src, "+to", dst], input=f"{x} {y}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


SWISSTOPO = ("swisstopo, Formulas and constants for the calculation of the Swiss conformal cylindrical "
             "projection (December 2016), section 3.2: the station Rigi")
PROJ = "PROJ 9.9.0 cs2cs applying EPSG:2056 from CH1903 geographic coordinates"

PLACES = [
    ("Rigi, swisstopo's example", 47 + 3 / 60 + 28.95659233 / 3600, 8 + 29 / 60 + 11.11127154 / 3600),
    ("Bern, the fundamental point", 46 + 57 / 60 + 8.66 / 3600, 7 + 26 / 60 + 22.50 / 3600),
    ("Zurich main station", 47.378100, 8.540200),
    ("Geneva, Jet d'Eau", 46.207400, 6.155800),
    ("Lugano", 46.005000, 8.951100),
    ("St. Gallen Abbey", 47.423100, 9.377400),
    ("Matterhorn summit", 45.976400, 7.658600),
    ("Basel Minster", 47.556100, 7.592300),
    ("Chur", 46.849100, 9.533000),
    ("Sion", 46.233100, 7.360800),
    ("Neuchatel", 46.992700, 6.931400),
    ("Davos", 46.801100, 9.836000),
]

rows_f, rows_i = [], []
for name, lat, lon in PLACES:
    e, n = cs2cs(CH1903, LV95, lon, lat)
    source = SWISSTOPO if name.startswith("Rigi") else f"{PROJ}: {name}"
    rows_f.append({
        "input": {"lat": lat, "lon": lon},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                   "result.lv03_easting.value": e - 2_000_000, "result.lv03_northing.value": n - 1_000_000},
        "source": source,
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102; swisstopo December 2016",
        "tolerance": {k: {"abs": 1e-3} for k in
                      ("result.easting.value", "result.northing.value",
                       "result.lv03_easting.value", "result.lv03_northing.value")},
    })
    blon, blat = cs2cs(LV95, CH1903, e, n)
    rows_i.append({
        "input": {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
        "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon,
                   "result.grid": "read as LV95"},
        "source": source + ", read back",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102; swisstopo December 2016",
        "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
    })

# The same point given in LV03, which the tool recognises and shifts.
e, n = cs2cs(CH1903, LV95, 8 + 29 / 60 + 11.11127154 / 3600, 47 + 3 / 60 + 28.95659233 / 3600)
blon, blat = cs2cs(LV95, CH1903, e, n)
rows_i.append({
    "input": {"easting": f"{e - 2_000_000:.6f} m", "northing": f"{n - 1_000_000:.6f} m"},
    "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon,
               "result.grid": "read as LV03 and shifted onto LV95",
               "meta.warnings.*.code": "INPUT_NORMALIZED"},
    "source": SWISSTOPO + ", given in the older LV03 grid, which the tool recognises by its six-figure easting",
    "sourceVersion": "PROJ 9.9.0; swisstopo December 2016",
    "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
})

# Added 2026-11-16 to bring both tools to the twenty vectors the stable bar
# asks for, appended after the published rows so their ids stay put: the
# corners and edges of the country as well as its towns.
EXTRA = [
    ("Lausanne Cathedral", 46.522500, 6.635100),
    ("Lucerne, the Chapel Bridge", 47.051600, 8.307500),
    ("Winterthur", 47.499900, 8.724100),
    ("Schaffhausen, the Munot", 47.696700, 8.638300),
    ("Locarno", 46.170800, 8.799500),
    ("Interlaken", 46.686300, 7.863200),
    ("Zermatt", 46.020700, 7.749100),
    ("Fribourg", 46.806500, 7.161900),
    ("St. Moritz", 46.497200, 9.838500),
    ("Chiasso, the southern border", 45.832400, 9.031200),
    ("Porrentruy, in the north-west", 47.415300, 7.075200),
    ("Mustair, at the eastern border", 46.629700, 10.449100),
    ("Rheinfelden, on the Rhine", 47.554200, 7.794000),
]
for name, lat, lon in EXTRA:
    e, n = cs2cs(CH1903, LV95, lon, lat)
    rows_f.append({
        "input": {"lat": lat, "lon": lon},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                   "result.lv03_easting.value": e - 2_000_000, "result.lv03_northing.value": n - 1_000_000},
        "source": f"{PROJ}: {name}",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {k: {"abs": 1e-3} for k in
                      ("result.easting.value", "result.northing.value",
                       "result.lv03_easting.value", "result.lv03_northing.value")},
    })
    blon, blat = cs2cs(LV95, CH1903, e, n)
    rows_i.append({
        "input": {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
        "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon,
                   "result.grid": "read as LV95"},
        "source": f"{PROJ}: {name}, read back",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
    })

print("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":"))
                for i, r in enumerate(rows_f, 1)))
with open(sys.argv[1], "w") as f:
    f.write("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":"))
                      for i, r in enumerate(rows_i, 1)) + "\n")
