#!/usr/bin/env python3
"""Golden vectors for geodesy.grid-ref.irish-forward and irish-inverse
(add-international-grids, geodesy/national-grids: "Irish Grid references
(one letter)").

Two oracles:

  * PROJ 9.9.0 through cs2cs: the Irish Grid projection of EPSG 29903, and
    for WGS 84 the seven-parameter transformation of EPSG operation 1954,
    written out as +towgs84 so the operation cannot drift to another one PROJ
    prefers; check_against_epsg() refuses to run if projinfo stops giving
    these numbers for 29903 and 1954.
  * A second lettering, written here from the grid's layout (A to Z without
    I, rows of five from the north-west, V at the false origin) rather than
    taken from the core.

    gen_irish.py FORWARD.jsonl INVERSE.jsonl
"""
import json
import subprocess
import sys

GRID = ("+proj=tmerc +lat_0=53.5 +lon_0=-8 +k=1.000035 +x_0=200000 +y_0=250000 "
        "+a=6377340.189 +rf=299.3249646 +units=m +no_defs")
TOWGS = "+towgs84=482.5,-130.6,564.6,-1.042,-0.214,-0.631,8.15"
TM75 = "+proj=longlat +a=6377340.189 +rf=299.3249646 +no_defs"
WGS = "+proj=longlat +datum=WGS84 +no_defs"
LETTERS = "ABCDEFGHJKLMNOPQRSTUVWXYZ"


def check_against_epsg():
    want = subprocess.run(["projinfo", "-o", "PROJ", "EPSG:29903"], check=True,
                          capture_output=True, text=True).stdout
    for p in GRID.split()[:-2]:
        assert p in want, (p, want)
    ops = subprocess.run(["projinfo", "-s", "EPSG:4300", "-t", "EPSG:4326", "-o", "PROJ"],
                         check=True, capture_output=True, text=True).stdout
    block = ops.split("EPSG:1954")[1].split("-----")[0]
    for p in ("+x=482.5", "+y=-130.6", "+z=564.6", "+rx=-1.042", "+ry=-0.214",
              "+rz=-0.631", "+s=8.15", "+convention=position_vector"):
        assert p in block, (p, block)


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10"] + src.split() + ["+to"] + dst.split(),
                         input=f"{x} {y}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def letter(e, n):
    col, row = int(e // 100000), 4 - int(n // 100000)
    return LETTERS[row * 5 + col]


def ref(e, n, figures):
    d = figures // 2
    unit = 100000 / 10 ** d
    part = lambda v: f"{int((v % 100000) // unit):0{d}d}"
    return f"{letter(e, n)} {part(e)} {part(n)}"


PLACES = [
    ("The Spire of Dublin", 53.349804, -6.260310),
    ("Belfast City Hall", 54.596400, -5.930100),
    ("Cork, St Fin Barre's Cathedral", 51.894400, -8.480300),
    ("Galway, Eyre Square", 53.274500, -9.049000),
    ("Carrauntoohil", 52.003900, -9.742500),
    ("Malin Head", 55.380000, -7.373600),
    ("Derry, the Guildhall", 54.996100, -7.319000),
    ("Mizen Head", 51.450000, -9.817000),
    ("Slieve Donard", 54.180200, -5.920700),
    ("Achill Head", 53.968000, -10.255000),
]


def rows():
    fwd, inv = [], []
    for place, lat, lon in PLACES:
        for datum in ("WGS84", "TM75"):
            src = WGS + " " if datum == "WGS84" else TM75
            if datum == "WGS84":
                e, n = cs2cs(WGS, GRID + " " + TOWGS, lon, lat)
            else:
                e, n = cs2cs(TM75, GRID, lon, lat)
            for figures in (10, 6):
                fwd.append({
                    "input": {"lat": lat, "lon": lon, "datum": datum, "figures": figures},
                    "expect": {"ok": True, "result.reference": ref(e, n, figures),
                               "result.square": letter(e, n),
                               "result.easting.value": e, "result.northing.value": n},
                    "source": f"PROJ 9.9.0 cs2cs applying EPSG 29903{' after EPSG 1954 from WGS 84' if datum == 'WGS84' else ' on TM75'}, lettered by gen_irish.py's own lettering: {place}",
                    "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                    "tolerance": {"result.easting.value": {"abs": 1e-3},
                                  "result.northing.value": {"abs": 1e-3}},
                })
        # Read back: the 6-figure reference on WGS 84, whose center PROJ
        # carries back through the grid and EPSG 1954.
        e, n = cs2cs(WGS, GRID + " " + TOWGS, lon, lat)
        r = ref(e, n, 6)
        side = 100.0
        se, sn = (e // side) * side, (n // side) * side
        clon, clat = cs2cs(GRID + " " + TOWGS, WGS, se + 50, sn + 50)
        inv.append({
            "input": {"reference": r, "datum": "WGS84"},
            "expect": {"ok": True, "result.easting.value": se, "result.northing.value": sn,
                       "result.square_size.value": side, "result.square": letter(e, n),
                       "result.lat.value": clat, "result.lon.value": clon},
            "source": f"gen_irish.py's own lettering for {r}, and PROJ 9.9.0 cs2cs carrying the square's center back through EPSG 29903 and EPSG 1954: {place}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.easting.value": {"abs": 0}, "result.northing.value": {"abs": 0},
                          "result.square_size.value": {"abs": 0},
                          "result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })
        clon, clat = cs2cs(GRID, TM75, se + 50, sn + 50)
        inv.append({
            "input": {"reference": r.replace(" ", ""), "datum": "TM75"},
            "expect": {"ok": True, "result.easting.value": se, "result.northing.value": sn,
                       "result.lat.value": clat, "result.lon.value": clon},
            "source": f"gen_irish.py's own lettering for {r}, written without spaces, and PROJ 9.9.0 cs2cs carrying the square's center back through EPSG 29903 onto TM75: {place}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.easting.value": {"abs": 0}, "result.northing.value": {"abs": 0},
                          "result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
        })
    # The convention's own cases: a two-figure 10 km square, a bare letter's
    # 100 km square, and the letter I, which the grid does not use.
    inv.append({
        "input": {"reference": "N 1 3", "datum": "TM75"},
        "expect": {"ok": True, "result.easting.value": 210000.0,
                   "result.northing.value": 230000.0, "result.square_size.value": 10000.0},
        "source": "The Irish Grid lettering: N is the 100 km square from E 200000 and N 200000, so N 1 3 is the 10 km square from E 210000 N 230000",
        "sourceVersion": "EPSG v13.102",
        "tolerance": {"result.easting.value": {"abs": 0}, "result.northing.value": {"abs": 0},
                      "result.square_size.value": {"abs": 0}},
    })
    inv.append({
        "input": {"reference": "I 15 34"},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": "The Irish Grid lettering leaves out I, as the National Grid does",
        "sourceVersion": "EPSG v13.102",
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
