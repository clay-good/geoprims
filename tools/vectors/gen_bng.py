#!/usr/bin/env python3
"""Golden vectors for geodesy.grid-ref.bng-forward and bng-inverse
(add-international-grids, geodesy/national-grids).

Two independent checks:

- The projection and the datum step come from PROJ 9.9.0 through cs2cs, with
  the National Grid written out as Ordnance Survey's guide gives it (Airy
  1830, true origin 49N 2W, scale 0.9996012717, false origin 400 km west and
  100 km north) and the seven-parameter transformation of EPSG 1314 as
  +towgs84, at the three decimals that operation carries (Ordnance Survey's
  own guide publishes a four-decimal set, which is a different published
  transformation), which is the path the tool takes without the OSTN15 grid. PROJ
  reproduces the guide's own Annexe C example, E 651409.9029 N 313177.2703,
  to a tenth of a millimetre.
- The lettering is written again here from the convention in the guide's
  section 7.1: 100 km squares, two letters, the letter I left out.

    gen_bng.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import subprocess

OSGB_LL = "+proj=longlat +ellps=airy +no_defs"
BNG = ("+proj=tmerc +lat_0=49 +lon_0=-2 +k=0.9996012717 +x_0=400000 +y_0=-100000 "
       "+ellps=airy +units=m +no_defs")
BNG_FROM_WGS = BNG + " +towgs84=446.448,-125.157,542.06,0.15,0.247,0.842,-20.489"
LETTERS = "ABCDEFGHJKLMNOPQRSTUVWXYZ"


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10", src, "+to", dst], input=f"{x} {y}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


def square(e, n):
    ce, cn = int(e // 100000), int(n // 100000)
    first = (19 - cn) - (19 - cn) % 5 + (ce + 10) // 5
    second = ((19 - cn) * 5) % 25 + ce % 5
    return LETTERS[first] + LETTERS[second]


def reference(e, n, figures):
    digits = figures // 2
    unit = 100000 / 10 ** digits
    part = lambda v: f"{int((v % 100000) // unit):0{digits}d}"
    return f"{square(e, n)} {part(e)} {part(n)}"


rows = []
GUIDE = "Ordnance Survey, A Guide to Coordinate Systems in Great Britain v3.6"
PROJ = "PROJ 9.9.0 cs2cs with the guide's National Grid parameters, and the lettering written again in Python"

# The guide's own Annexe C worked example, on OSGB36.
e, n = cs2cs(OSGB_LL, BNG, 1.717921583, 52.657570305)
rows.append({
    "input": {"lat": 52.657570305, "lon": 1.717921583, "datum": "OSGB36", "figures": 10},
    "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
               "result.reference": reference(e, n, 10), "result.square": square(e, n)},
    "source": GUIDE + " Annexe C: the worked example gives E 651409.903 N 313177.270, which PROJ reproduces to a tenth of a millimetre",
    "sourceVersion": "v3.6 (2020); PROJ 9.9.0",
    "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
})

# Places across Great Britain, on WGS 84 (the seven-parameter path) and on
# OSGB36 itself, at several precisions.
PLACES = [
    ("Edinburgh Castle", 55.948611, -3.199722, 10),
    ("Cardiff Castle", 51.482222, -3.181111, 8),
    ("Snowdon summit", 53.068500, -4.076100, 10),
    ("Land's End", 50.066389, -5.714722, 6),
    ("John o' Groats", 58.643889, -3.070000, 6),
    ("York Minster", 53.962222, -1.081944, 10),
    ("Ben Nevis", 56.796850, -5.003600, 8),
    ("Norwich Castle", 52.628300, 1.294400, 10),
    ("Isle of Man, Douglas", 54.152500, -4.486400, 4),
    ("Dover Castle", 51.129400, 1.321100, 2),
]
for name, lat, lon, figures in PLACES:
    for datum in ("WGS84", "OSGB36"):
        src, dst = (("EPSG:4326", BNG_FROM_WGS) if datum == "WGS84" else (OSGB_LL, BNG))
        e, n = cs2cs(src, dst, *((lat, lon) if datum == "WGS84" else (lon, lat)))
        rows.append({
            "input": {"lat": lat, "lon": lon, "datum": datum, "figures": figures},
            "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                       "result.reference": reference(e, n, figures)},
            "source": f"{PROJ}: {name} on {datum}",
            "sourceVersion": "PROJ 9.9.0",
            "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
        })

# Reading a reference back: the square it names, and its centre.
READS = [
    ("TQ 30 80", 530000.0, 180000.0, 1000.0),
    ("TQ", 500000.0, 100000.0, 100000.0),
    ("SV 00 00", 0.0, 0.0, 1000.0),
    ("TG 51409 13177", 651409.0, 313177.0, 1.0),
    ("NT 2746 7355", 327460.0, 673550.0, 10.0),
    ("HU 40 40", 440000.0, 1140000.0, 1000.0),
]
for ref, e, n, side in READS:
    clat, clon = cs2cs(BNG, OSGB_LL, e + side / 2, n + side / 2)
    rows.append({
        "input": {"reference": ref, "datum": "OSGB36"},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                   "result.square_size.value": side,
                   "result.lat.value": clon, "result.lon.value": clat},
        "source": f"{PROJ}: the square {ref} names, and its centre on OSGB36",
        "sourceVersion": "PROJ 9.9.0",
        "tolerance": {k: {"abs": 1e-6} for k in
                      ("result.easting.value", "result.northing.value", "result.square_size.value",
                       "result.lat.value", "result.lon.value")},
    })

# Refusals, each from the convention.
for ref, why in [
    ("TI 30 80", "the letter I is not used"),
    ("TQ 300 80", "an odd number of figures names no square"),
    ("ZZ 30 80", "ZZ is not a square of the National Grid"),
    ("TQ 3A 80", "after the letters a reference is all figures"),
]:
    rows.append({"input": {"reference": ref}, "expect": {"ok": False, "error.code": "INVALID_INPUT"},
                 "source": GUIDE + f" section 7.1: {why}", "sourceVersion": "v3.6 (2020)", "tolerance": {}})
rows.append({"input": {"lat": 48.0, "lon": -2.0, "datum": "OSGB36", "figures": 10},
             "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
             "source": GUIDE + " section 7.1: south of the lettered grid there is no reference to give",
             "sourceVersion": "v3.6 (2020)", "tolerance": {}})

# Added 2026-11-16 to bring bng-inverse to the twenty vectors the stable bar
# asks for. `gen_bng.py --append-inverse INVERSE` appends these after the
# published rows, numbering on, and refuses if they are already there: a
# reference at every precision from real places, read back to the square it
# names and that square's centre, on OSGB36 and on WGS 84.
import sys
WGS_LL = "+proj=longlat +datum=WGS84 +no_defs"
EXTRA_READS = [
    ("Ben Nevis", 56.796900, -5.003600, 6, "OSGB36"),
    ("Snowdon", 53.068500, -4.076200, 8, "OSGB36"),
    ("Land's End", 50.066000, -5.714700, 4, "OSGB36"),
    ("Lerwick", 60.155000, -1.145000, 10, "OSGB36"),
    ("Edinburgh Castle", 55.948600, -3.199900, 2, "OSGB36"),
    ("Cardiff Castle", 51.482200, -3.181200, 6, "WGS84"),
    ("York Minster", 53.962100, -1.081900, 8, "WGS84"),
    ("St Mary's, Isles of Scilly", 49.915200, -6.316000, 6, "WGS84"),
    ("Stornoway", 58.209000, -6.386500, 4, "WGS84"),
    ("Dover Castle", 51.129400, 1.322400, 10, "WGS84"),
    ("The Lizard", 49.959800, -5.205400, 6, "OSGB36"),
]
if len(sys.argv) == 3 and sys.argv[1] == "--append-inverse":
    path = sys.argv[2]
    if any(name in open(path).read() for name, *_ in EXTRA_READS):
        sys.exit(f"{path} already holds the added places")
    count = sum(1 for _ in open(path))
    with open(path, "a") as f:
        for k, (name, lat, lon, figures, datum) in enumerate(EXTRA_READS):
            (ll, grid) = (OSGB_LL, BNG) if datum == "OSGB36" else (WGS_LL, BNG_FROM_WGS)
            e, n = cs2cs(ll, grid, lon, lat)
            ref = reference(e, n, figures)
            side = 100000 / 10 ** (figures // 2)
            se, sn = (e // side) * side, (n // side) * side
            clon, clat = cs2cs(grid, ll, se + side / 2, sn + side / 2)
            f.write(json.dumps({"id": f"v{count + k + 1:03d}",
                "input": {"reference": ref, "datum": datum},
                "expect": {"ok": True, "result.easting.value": se, "result.northing.value": sn,
                           "result.square_size.value": side, "result.square": square(e, n),
                           "result.lat.value": clat, "result.lon.value": clon},
                "source": f"{PROJ}: {name}'s {figures}-figure reference {ref}, the square it names, and its centre on {datum}",
                "sourceVersion": "PROJ 9.9.0",
                "tolerance": {k2: {"abs": 1e-6} for k2 in
                              ("result.easting.value", "result.northing.value", "result.square_size.value",
                               "result.lat.value", "result.lon.value")},
            }, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.exit(0)

for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
