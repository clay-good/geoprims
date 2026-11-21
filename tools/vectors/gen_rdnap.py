#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.rdnaptrans and its inverse
(add-international-grids, geodesy/national-grids: "Canada, Europe, and the
other national paths", the RDNAPTRANS2018 half).

The oracle is PROJ 9.9.0 applying NSGI's own grids, the same two the asset is
packed from (pinned in tools/data/rdnaptrans-assets.mjs): the rdtrans2018
correction grid through hgridshift, the NLGEO2018 quasi-geoid through
vgridshift, and the RD map projection through sterea with EPSG 28992's
parameters. The grid convergence and the point scale factor come from PROJ's
own `proj -V` factors at the real RD Bessel point, so nothing in the answer is
left unpinned. The first vector of each file is the specification's NSGI check
point.

    gen_rdnap.py GRID_DIR OUT_FORWARD.jsonl OUT_INVERSE.jsonl
"""
import json
import subprocess
import sys

GRID_DIR = sys.argv[1]
HGRID = f"{GRID_DIR}/nl_nsgi_rdtrans2018.tif"
VGRID = f"{GRID_DIR}/nl_nsgi_nlgeo2018.tif"
STEREA = ("+proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 "
          "+k=0.9999079 +x_0=155000 +y_0=463000 +ellps=bessel").split()

PIPELINE = [
    "+proj=pipeline",
    "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
    "+step", "+inv", "+proj=vgridshift", f"+grids={VGRID}", "+multiplier=1",
    "+step", "+inv", "+proj=hgridshift", f"+grids={HGRID}",
    "+step", *STEREA,
]

# ETRS89 to real RD Bessel alone, to pin the point the projection acts at.
TO_BESSEL = [
    "+proj=pipeline",
    "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
    "+step", "+inv", "+proj=hgridshift", f"+grids={HGRID}",
    "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg",
]


def run(args, line):
    out = subprocess.run(args, input=line, check=True, capture_output=True, text=True)
    return out.stdout.split()


def forward(lat, lon, h):
    """RD easting, northing and NAP height."""
    o = run(["cct", "-d", "9", *PIPELINE], f"{lon} {lat} {h} 0\n")
    return float(o[0]), float(o[1]), float(o[2])


def inverse(x, y, nap):
    """ETRS89 latitude, longitude and ellipsoidal height."""
    o = run(["cct", "-d", "11", "-I", *PIPELINE], f"{x} {y} {nap} 0\n")
    return float(o[1]), float(o[0]), float(o[2])


def bessel(lat, lon):
    o = run(["cct", "-d", "11", *TO_BESSEL], f"{lon} {lat} 0 0\n")
    return float(o[1]), float(o[0])


def factors(lat, lon):
    """PROJ's own convergence (degrees) and point scale at a Bessel point."""
    out = subprocess.run(["proj", "-V", *STEREA], input=f"{lon} {lat}\n",
                         check=True, capture_output=True, text=True).stdout
    conv = scale = None
    for line in out.splitlines():
        if line.startswith("Convergence"):
            conv = float(line.split("[")[1].split("]")[0])
        elif line.startswith("Meridian scale (h)"):
            scale = float(line.split(":")[1].split("(")[0])
    if conv is None or scale is None:
        raise SystemExit(f"no factors for {lat} {lon}:\n{out}")
    return conv, scale


def geoid(lat, lon):
    """NLGEO2018's quasi-geoid height, from the height PROJ takes off."""
    o = run(["cct", "-d", "9", "+proj=pipeline",
             "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
             "+step", "+inv", "+proj=vgridshift", f"+grids={VGRID}", "+multiplier=1"],
            f"{lon} {lat} 0 0\n")
    return -float(o[2])


# Places across the Netherlands and its EEZ, with a plausible ellipsoidal
# height, chosen to cover both the dense subgrid and the coarse parent.
PLACES = [
    ("Amsterdam, Dam Square", 52.373056, 4.892778, 45.0),
    ("Rotterdam, Erasmusbrug", 51.909400, 4.486800, 43.5),
    ("The Hague, Binnenhof", 52.079700, 4.312800, 44.0),
    ("Utrecht, Dom Tower", 52.090800, 5.121400, 43.5),
    ("Eindhoven station", 51.443300, 5.479200, 46.0),
    ("Groningen, Martinitoren", 53.219400, 6.568100, 45.5),
    ("Maastricht, Sint Servaas", 50.848900, 5.688900, 92.0),
    ("Arnhem, John Frost Bridge", 51.976900, 5.906900, 56.0),
    ("Leeuwarden, Oldehove", 53.202200, 5.790600, 44.5),
    ("Middelburg, Lange Jan", 51.498900, 3.613100, 41.0),
    ("Enschede station", 52.223300, 6.890000, 48.0),
    ("Den Helder, Lange Jaap", 52.961900, 4.722800, 44.0),
    ("Vlissingen tide gauge", 51.442200, 3.596100, 41.0),
    ("Terschelling, Brandaris", 53.362800, 5.213300, 44.0),
    ("Amersfoort, the central point", 52.156161, 5.387639, 43.0),
    ("Nijmegen, Waalbrug", 51.853600, 5.864400, 55.0),
    ("Breda, Grote Kerk", 51.588300, 4.775600, 46.0),
    ("Apeldoorn, Het Loo", 52.234400, 5.945800, 68.0),
    ("Lelystad, Batavia", 52.527500, 5.433100, 40.5),
    ("Rottumerplaat", 53.546000, 6.567000, 44.0),
    # Out in the EEZ, where only the coarse parent grid reaches.
    ("North Sea, Borssele wind farm", 51.700000, 3.050000, 43.0),
    ("North Sea, Dogger Bank edge", 54.800000, 3.400000, 43.0),
    ("North Sea, off Ameland", 54.200000, 5.600000, 43.0),
    ("North Sea, south-west corner", 50.300000, 2.300000, 43.0),
    ("North Sea, north-east corner", 55.700000, 7.700000, 43.0),
    ("North Sea, west of the dense subgrid", 51.800000, 2.200000, 43.0),
]

# Just outside the parent grid on each side. The grids reach well past the
# Netherlands, so Belgium and the German bight are inside them.
OUT_OF_DOMAIN = [
    ("Paris, south of the grid", 48.858200, 2.294500),
    ("Hamburg, east of the grid", 53.550000, 9.993000),
    ("Dunkirk, a tenth of a degree west of the grid's 2\u00b0E edge", 51.034800, 1.900000),
    ("Just south of the grid's 50\u00b0N edge", 49.950000, 5.000000),
]


def dump(rows):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                separators=(",", ":"))
                     for i, r in enumerate(rows, 1)) + "\n"


def main():
    spec_x, spec_y, spec_nap = 128410.0958, 445806.4960, -0.4754
    fwd = [{
        "input": {"lat": 52.0, "lon": 5.0, "height": "43 m"},
        "expect": {"ok": True, "result.easting.value": spec_x,
                   "result.northing.value": spec_y, "result.nap_height.value": spec_nap},
        "source": "The national-grids specification's NSGI check point: 5°E 52°N at h 43 m on ETRS89 gives RD 128410.0958, 445806.496 and −0.4754 m NAP",
        "sourceVersion": "RDNAPTRANS2018, 27 June 2022",
        "tolerance": {"result.easting.value": {"abs": 0.0001},
                      "result.northing.value": {"abs": 0.0001},
                      "result.nap_height.value": {"abs": 0.0001}},
    }]
    inv = [{
        "input": {"easting": f"{spec_x} m", "northing": f"{spec_y} m",
                  "nap_height": f"{spec_nap} m"},
        "expect": {"ok": True, "result.lat.value": 52.0, "result.lon.value": 5.0,
                   "result.height.value": 43.0},
        "source": "The national-grids specification's NSGI check point the other way: RD 128410.0958, 445806.496 at −0.4754 m NAP gives 52°N 5°E at h 43 m on ETRS89",
        "sourceVersion": "RDNAPTRANS2018, 27 June 2022",
        "tolerance": {"result.lat.value": {"abs": 2e-9}, "result.lon.value": {"abs": 2e-9},
                      "result.height.value": {"abs": 0.0002}},
    }]

    for name, lat, lon, h in PLACES:
        x, y, nap = forward(lat, lon, h)
        blat, blon = bessel(lat, lon)
        conv, scale = factors(blat, blon)
        n = geoid(lat, lon)
        fwd.append({
            "input": {"lat": lat, "lon": lon, "height": f"{h} m"},
            "expect": {"ok": True, "result.easting.value": x, "result.northing.value": y,
                       "result.nap_height.value": nap, "result.geoid_height.value": n,
                       "result.convergence.value": conv, "result.scale_factor": scale},
            "source": f"PROJ 9.9.0 applying NSGI's rdtrans2018, NLGEO2018 and EPSG 28992's sterea: {name}; the convergence and scale from proj -V at the real RD Bessel point",
            "sourceVersion": "PROJ 9.9.0; RDNAPTRANS2018 grids of 2018-11-22 and 2019-03-22",
            "tolerance": {"result.easting.value": {"abs": 0.0001},
                          "result.northing.value": {"abs": 0.0001},
                          "result.nap_height.value": {"abs": 0.0001},
                          "result.geoid_height.value": {"abs": 0.0001},
                          "result.convergence.value": {"abs": 1e-7},
                          "result.scale_factor": {"abs": 1e-8}},
        })

    for name, lat, lon, h in PLACES:
        x, y, nap = forward(lat, lon, h)
        la, lo, hh = inverse(round(x, 4), round(y, 4), round(nap, 4))
        n = geoid(la, lo)
        inv.append({
            "input": {"easting": f"{round(x, 4)} m", "northing": f"{round(y, 4)} m",
                      "nap_height": f"{round(nap, 4)} m"},
            "expect": {"ok": True, "result.lat.value": la, "result.lon.value": lo,
                       "result.height.value": hh, "result.geoid_height.value": n},
            "source": f"PROJ 9.9.0 applying the same NSGI grids in reverse: {name}",
            "sourceVersion": "PROJ 9.9.0; RDNAPTRANS2018 grids of 2018-11-22 and 2019-03-22",
            "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9},
                          "result.height.value": {"abs": 0.0001},
                          "result.geoid_height.value": {"abs": 0.0001}},
        })

    for name, lat, lon in OUT_OF_DOMAIN:
        fwd.append({
            "input": {"lat": lat, "lon": lon, "height": "43 m"},
            "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
            "source": f"NSGI's grids cover the European Netherlands and its EEZ, 50°N to 56°N and 2°E to 8°E: {name}",
            "sourceVersion": "RDNAPTRANS2018, 27 June 2022",
            "tolerance": {},
        })
    inv.append({
        "input": {"easting": "1500000 m", "northing": "463000 m", "nap_height": "0 m"},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": "1,345 km east of Amersfoort is far outside the correction grid, so there is no ETRS89 position to give",
        "sourceVersion": "RDNAPTRANS2018, 27 June 2022",
        "tolerance": {},
    })

    with open(sys.argv[2], "w") as f:
        f.write(dump(fwd))
    with open(sys.argv[3], "w") as f:
        f.write(dump(inv))


if __name__ == "__main__":
    main()
