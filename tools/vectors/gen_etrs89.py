#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.etrs89-forward and etrs89-inverse
(add-international-grids, geodesy/national-grids: "ETRS89-LAEA and LCC").

Sources:

  * IOGP Guidance Note 7-2 (September 2019), section 3.3.2: the ETRS-LAEA
    worked example as printed, both ways.
  * PROJ 9.9.0 through cs2cs applying EPSG 3035 and 3034 from ETRS89 (4258),
    checked against projinfo on every run: the coordinates at a millimetre.
  * PROJ's own `proj -V` factors for the same definitions: the convergence,
    the scale along the meridian, and the scale along the parallel, printed to
    eight decimals. LAEA is not conformal, so the two scales differ.

    gen_etrs89.py FORWARD.jsonl INVERSE.jsonl
"""
import json
import re
import subprocess
import sys

LAEA = "+proj=laea +lat_0=52 +lon_0=10 +x_0=4321000 +y_0=3210000 +ellps=GRS80"
LCC = "+proj=lcc +lat_0=52 +lon_0=10 +lat_1=35 +lat_2=65 +x_0=4000000 +y_0=2800000 +ellps=GRS80"
GRIDS = {"LAEA Europe": (3035, LAEA), "LCC Europe": (3034, LCC)}


def check_against_epsg():
    for name, (code, p4) in GRIDS.items():
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{code}"], check=True,
                              capture_output=True, text=True).stdout
        for p in p4.split():
            assert p in want, (code, p, want)


def cs2cs_fwd(code, lat, lon):
    out = subprocess.run(["cs2cs", "-d", "10", "EPSG:4258", "+to", f"EPSG:{code}"],
                         input=f"{lat} {lon}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    # EPSG 3035 and 3034 declare northing then easting.
    return float(out[1]), float(out[0])


def cs2cs_inv(code, e, n):
    out = subprocess.run(["cs2cs", "-d", "12", f"EPSG:{code}", "+to", "EPSG:4258"],
                         input=f"{n} {e}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def factors(p4, lat, lon):
    out = subprocess.run(["proj", "-V"] + p4.split(), input=f"{lon} {lat}\n", check=True,
                         capture_output=True, text=True).stdout
    num = lambda label: float(re.search(label + r"[^:]*:\s*([-\d.]+)", out).group(1))
    conv = float(re.search(r"Convergence[^\[]*\[\s*([-\d.]+)\s*\]", out).group(1))
    return conv, num(r"Meridian scale \(h\)"), num(r"Parallel scale \(k\)")


PLACES = [
    ("Lisbon", 38.7223, -9.1393),
    ("Madrid, Puerta del Sol", 40.4168, -3.7038),
    ("Reykjavík", 64.1466, -21.9426),
    ("Dublin", 53.3498, -6.2603),
    ("Paris, Notre-Dame", 48.8530, 2.3499),
    ("Brussels, the Grand-Place", 50.8467, 4.3525),
    ("Frankfurt", 50.1109, 8.6821),
    ("The origin, 52°N 10°E", 52.0, 10.0),
    ("Rome, the Colosseum", 41.8902, 12.4922),
    ("Stockholm", 59.3293, 18.0686),
    ("Warsaw", 52.2297, 21.0122),
    ("Athens, the Acropolis", 37.9715, 23.7257),
    ("Helsinki", 60.1699, 24.9384),
    ("Nicosia, Cyprus", 35.1856, 33.3823),
    ("Tromsø", 69.6492, 18.9553),
    ("Ponta Delgada, Azores", 37.7412, -25.6756),
    ("Longyearbyen, Svalbard", 78.2232, 15.6267),
]


def rows():
    fwd, inv = [], []
    lat0 = 50.0
    fwd.append({
        "input": {"lat": 50, "lon": 5},
        "expect": {"ok": True, "result.easting.value": 3962799.45,
                   "result.northing.value": 2999718.85, "result.grid": "LAEA Europe"},
        "source": "IOGP Guidance Note 7-2 (September 2019), section 3.3.2, Lambert Azimuthal Equal Area: the ETRS89 / ETRS-LAEA example, 50°N 5°E, as printed",
        "sourceVersion": "IOGP 373-7-2, revised September 2019",
        "tolerance": {"result.easting.value": {"abs": 0.005}, "result.northing.value": {"abs": 0.005}},
    })
    inv.append({
        "input": {"easting": "3962799.45 m", "northing": "2999718.85 m"},
        "expect": {"ok": True, "result.lat.value": lat0, "result.lon.value": 5.0},
        "source": "IOGP Guidance Note 7-2 (September 2019), section 3.3.2, reverse: E 3962799.45 m, N 2999718.85 m on ETRS89 / ETRS-LAEA back to 50°N 5°E, to the half centimetre the printed coordinates carry",
        "sourceVersion": "IOGP 373-7-2, revised September 2019",
        "tolerance": {"result.lat.value": {"abs": 1e-7}, "result.lon.value": {"abs": 1e-7}},
    })
    for grid, (code, p4) in GRIDS.items():
        for place, lat, lon in PLACES:
            e, n = cs2cs_fwd(code, lat, lon)
            inp = {"lat": lat, "lon": lon}
            if grid != "LAEA Europe":
                inp["grid"] = grid
            fwd.append({
                "input": inp,
                "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                           "result.grid": grid},
                "source": f"PROJ 9.9.0 cs2cs applying EPSG:{code} ({grid}): {place}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": {"result.easting.value": {"abs": 1e-3},
                              "result.northing.value": {"abs": 1e-3}},
            })
            blat, blon = cs2cs_inv(code, e, n)
            ii = {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"}
            if grid != "LAEA Europe":
                ii["grid"] = grid
            inv.append({
                "input": ii,
                "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
                "source": f"PROJ 9.9.0 cs2cs applying EPSG:{code} ({grid}) in reverse: {place}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
            })
    for grid, (code, p4) in GRIDS.items():
        for place, lat, lon in [("Lisbon", 38.7223, -9.1393), ("Helsinki", 60.1699, 24.9384),
                                ("Ponta Delgada, Azores", 37.7412, -25.6756),
                                ("Longyearbyen, Svalbard", 78.2232, 15.6267)]:
            conv, h, k = factors(p4, lat, lon)
            inp = {"lat": lat, "lon": lon}
            if grid != "LAEA Europe":
                inp["grid"] = grid
            fwd.append({
                "input": inp,
                "expect": {"ok": True, "result.convergence.value": conv,
                           "result.scale_meridian": h, "result.scale_parallel": k},
                "source": f"PROJ 9.9.0 proj -V on EPSG:{code}'s definition ({grid}): the convergence and the scales along the meridian and the parallel at {place}, printed to eight decimals",
                "sourceVersion": "PROJ 9.9.0",
                "tolerance": {"result.convergence.value": {"abs": 1e-7},
                              "result.scale_meridian": {"abs": 1e-8},
                              "result.scale_parallel": {"abs": 1e-8}},
            })
    return fwd, inv


def dump(rs):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                separators=(",", ":"))
                     for i, r in enumerate(rs, 1)) + "\n"


def main():
    check_against_epsg()
    fwd, inv = rows()
    if len(sys.argv) != 3:
        sys.stdout.write(dump(fwd) + "--- inverse ---\n" + dump(inv))
        return
    with open(sys.argv[1], "w") as f:
        f.write(dump(fwd))
    with open(sys.argv[2], "w") as f:
        f.write(dump(inv))


if __name__ == "__main__":
    main()
