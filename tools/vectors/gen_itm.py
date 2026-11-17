#!/usr/bin/env python3
"""Golden vectors for geodesy.grid-ref.itm-forward and itm-inverse
(add-international-grids, geodesy/national-grids).

The oracle is PROJ 9.9.0 through cs2cs applying EPSG:2157 itself, so the
parameters come from the EPSG dataset rather than from this repository.

    gen_itm.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import subprocess

ITM = "EPSG:2157"
ETRS = "+proj=longlat +ellps=GRS80 +no_defs"


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10", src, "+to", dst], input=f"{x} {y}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


PLACES = [
    ("The Spire of Dublin", 53.349806, -6.260247),
    ("Cork, Shandon Bells", 51.902100, -8.473400),
    ("Galway Cathedral", 53.275800, -9.057200),
    ("Belfast City Hall", 54.596400, -5.930000),
    ("Derry, the walls", 54.995800, -7.321700),
    ("Killarney National Park", 52.012000, -9.508000),
    ("Malin Head", 55.381400, -7.372500),
    ("Mizen Head", 51.450300, -9.812400),
    ("Carrauntoohil summit", 51.999400, -9.742500),
    ("Wexford quay", 52.335300, -6.457600),
    ("Sligo Abbey", 54.270000, -8.472000),
    ("Athlone Castle", 53.423300, -7.941100),
]

rows_f, rows_i = [], []
for name, lat, lon in PLACES:
    e, n = cs2cs(ETRS, ITM, lon, lat)
    rows_f.append({
        "input": {"lat": lat, "lon": lon},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:2157: {name}",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
    })
    blon, blat = cs2cs(ITM, ETRS, e, n)
    rows_i.append({
        "input": {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
        "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:2157 in reverse: {name}",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
    })

# The grid's own origin and false origin, which pin the parameters themselves.
e, n = cs2cs(ETRS, ITM, -8.0, 53.5)
rows_f.append({
    "input": {"lat": 53.5, "lon": -8.0},
    "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
               "result.scale_factor": 0.99982},
    "source": "EPSG:2157's own origin: on the central meridian at the latitude of origin the easting is the false easting, the northing the false northing, and the scale factor the one the projection declares",
    "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
    "tolerance": {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
                  "result.scale_factor": {"abs": 1e-12}},
})
rows_i.append({
    "input": {"easting": "600000 m", "northing": "750000 m"},
    "expect": {"ok": True, "result.lat.value": 53.5, "result.lon.value": -8.0},
    "source": "EPSG:2157's own false origin reads back as the latitude of origin on the central meridian",
    "sourceVersion": "EPSG v13.102",
    "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
})

import sys

# Added 2026-11-16 to bring both tools to the twenty vectors the stable bar
# asks for. `gen_itm.py --append FORWARD INVERSE` appends these after the rows
# the published files already hold (which include five GeographicLib rows
# added by hand), numbering on, and refuses if they are already there.
EXTRA = [
    ("Limerick, King John's Castle", 52.669700, -8.625700),
    ("Waterford, Reginald's Tower", 52.260600, -7.105600),
    ("Kilkenny Castle", 52.650300, -7.249300),
    ("Dundalk", 54.001000, -6.404900),
    ("Achill Island, Keem Bay", 53.966600, -10.193800),
    ("Ballycastle, County Antrim", 55.204300, -6.244100),
    ("Dingle", 52.140800, -10.268800),
    ("Rosslare Harbour", 52.252600, -6.338200),
]
if len(sys.argv) == 4 and sys.argv[1] == "--append":
    fwd_path, inv_path = sys.argv[2], sys.argv[3]
    for path in (fwd_path, inv_path):
        if any(name in open(path).read() for name, _, _ in EXTRA):
            sys.exit(f"{path} already holds the added places")
    nf = sum(1 for _ in open(fwd_path))
    ni = sum(1 for _ in open(inv_path))
    with open(fwd_path, "a") as ff, open(inv_path, "a") as fi:
        for k, (name, lat, lon) in enumerate(EXTRA):
            e, n = cs2cs(ETRS, ITM, lon, lat)
            ff.write(json.dumps({"id": f"v{nf + k + 1:03d}",
                "input": {"lat": lat, "lon": lon},
                "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n},
                "source": f"PROJ 9.9.0 cs2cs applying EPSG:2157: {name}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
            }, ensure_ascii=False, separators=(",", ":")) + "\n")
            blon, blat = cs2cs(ITM, ETRS, e, n)
            fi.write(json.dumps({"id": f"v{ni + k + 1:03d}",
                "input": {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
                "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
                "source": f"PROJ 9.9.0 cs2cs applying EPSG:2157 in reverse: {name}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
            }, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.exit(0)

print("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":"))
                for i, r in enumerate(rows_f, 1)))
with open(sys.argv[1] if len(sys.argv) > 1 else "/dev/null", "w") as f:
    f.write("\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":"))
                      for i, r in enumerate(rows_i, 1)) + "\n")
