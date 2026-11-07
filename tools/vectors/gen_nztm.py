#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.nztm-forward and nztm-inverse
(add-international-grids, geodesy/national-grids).

The oracle is PROJ 9.9.0 through cs2cs applying EPSG:2193 itself, so the
projection's parameters come from the EPSG dataset rather than from this
repository.

    gen_nztm.py FORWARD.jsonl INVERSE.jsonl

Writes nothing without paths: published files stay frozen and new vectors are
appended by hand.
"""
import json
import subprocess
import sys

# EPSG:2193 declares its axes north then east, and cs2cs honours that, so
# asking for the CRS by code returns the northing first. The grid is taken
# through the PROJ.4 rendering of the same EPSG record instead, which is always
# easting then northing; projinfo EPSG:2193 prints it, and the parameters still
# come from the EPSG dataset rather than from this repository.
NZTM = ("+proj=tmerc +lat_0=0 +lon_0=173 +k=0.9996 +x_0=1600000 +y_0=10000000 "
        "+ellps=GRS80 +units=m +no_defs")
NZGD = "+proj=longlat +ellps=GRS80 +no_defs"


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10", src, "+to", dst], input=f"{x} {y}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


def check_against_the_epsg_code():
    """The PROJ.4 string above must be EPSG:2193, axis order aside."""
    want = subprocess.run(["projinfo", "-o", "PROJ", "EPSG:2193"], check=True,
                          capture_output=True, text=True).stdout
    for p in ("+proj=tmerc", "+lat_0=0", "+lon_0=173", "+k=0.9996",
              "+x_0=1600000", "+y_0=10000000", "+ellps=GRS80"):
        assert p in want, f"projinfo no longer gives {p} for EPSG:2193: {want}"


PLACES = [
    ("The Beehive, Wellington", -41.278611, 174.776944),
    ("Auckland, the Sky Tower", -36.848461, 174.762191),
    ("Christchurch, Cathedral Square", -43.530955, 172.636645),
    ("Dunedin, the Octagon", -45.873889, 170.503611),
    ("Hamilton", -37.787000, 175.279000),
    ("Tauranga", -37.686100, 176.166600),
    ("Napier", -39.492800, 176.912200),
    ("New Plymouth", -39.055600, 174.075800),
    ("Nelson", -41.270800, 173.283900),
    ("Invercargill", -46.413100, 168.352800),
    ("Queenstown", -45.031200, 168.662600),
    ("Aoraki / Mount Cook summit", -43.595100, 170.141900),
    ("Cape Reinga", -34.425900, 172.681300),
    ("Bluff", -46.598900, 168.337200),
    ("East Cape, the far east of the grid", -37.688900, 178.548600),
]

check_against_the_epsg_code()
rows_f, rows_i = [], []
for name, lat, lon in PLACES:
    e, n = cs2cs(NZGD, NZTM, lon, lat)
    rows_f.append({
        "input": {"lat": lat, "lon": lon},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:2193: {name}",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
    })
    blon, blat = cs2cs(NZTM, NZGD, e, n)
    rows_i.append({
        "input": {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
        "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:2193 in reverse: {name}",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
    })

# The grid's own origin, which pins the parameters rather than a place: on the
# central meridian at the latitude of origin the coordinates must be the false
# origin exactly, and the scale factor the one the standard declares.
e, n = cs2cs(NZGD, NZTM, 173.0, 0.0)
rows_f.append({
    "input": {"lat": 0, "lon": 173},
    "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
               "result.scale_factor": 0.9996},
    "source": "EPSG:2193's own origin: on the central meridian at the latitude of origin the easting is the false easting, the northing the false northing, and the scale factor the declared 0.9996",
    "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
    "tolerance": {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
                  "result.scale_factor": {"abs": 1e-12}},
})
rows_i.append({
    "input": {"easting": "1600000 m", "northing": "10000000 m"},
    "expect": {"ok": True, "result.lat.value": 0.0, "result.lon.value": 173.0},
    "source": "EPSG:2193's own false origin reads back as the latitude of origin on the central meridian",
    "sourceVersion": "EPSG v13.102",
    "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
})


def dump(rows):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                separators=(",", ":"))
                     for i, r in enumerate(rows, 1)) + "\n"


def main():
    if len(sys.argv) != 3:
        sys.stdout.write(dump(rows_f))
        sys.stdout.write("--- inverse ---\n")
        sys.stdout.write(dump(rows_i))
        return
    with open(sys.argv[1], "w") as f:
        f.write(dump(rows_f))
    with open(sys.argv[2], "w") as f:
        f.write(dump(rows_i))


if __name__ == "__main__":
    main()
