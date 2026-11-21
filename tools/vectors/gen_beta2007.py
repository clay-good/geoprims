#!/usr/bin/env python3
"""Golden vectors for geodesy.datum.beta2007 and its inverse
(add-international-grids, geodesy/national-grids: "Canada, Europe, and the
other national paths", the BeTA2007 half).

Two oracles. The first vectors of each file are AdV's own published test
records (BeTA2007 documentation v1.5, section 4.2), which the documentation
prints for its first two points: the same ground point on two Gauss-Krüger
zones, each carried to ETRS89 UTM zone 32, to a micrometre. The rest are
PROJ 9.9.0 applying the same grid through hgridshift between the zones'
own EPSG records, both ways.

    gen_beta2007.py GRID_TIF OUT_FORWARD.jsonl OUT_INVERSE.jsonl
"""
import json
import subprocess
import sys

TIF = sys.argv[1]


def gk(z):
    return f"+proj=tmerc +lat_0=0 +lon_0={3 * z} +k=1 +x_0={z}500000 +y_0=0 +ellps=bessel".split()


def utm(z):
    return f"+proj=utm +zone={z} +ellps=GRS80".split()


def run(src, east, north, dst, reverse=False):
    shift = ["+step"] + (["+inv"] if reverse else []) + ["+proj=hgridshift", f"+grids={TIF}"]
    pipe = ["+proj=pipeline", "+step", "+inv", *src, *shift, "+step", *dst]
    o = subprocess.run(["cct", "-d", "6", *pipe], input=f"{east} {north} 0 0\n",
                       check=True, capture_output=True, text=True).stdout.split()
    return float(o[0]), float(o[1])


def to_utm(gz, east, north, uz):
    return run(gk(gz), east, north, utm(uz))


def to_gk(uz, east, north, gz):
    return run(utm(uz), east, north, gk(gz), reverse=True)


def dhdn_lonlat(gz, east, north):
    o = subprocess.run(["cct", "-d", "12", "+proj=pipeline", "+step", "+inv", *gk(gz),
                        "+step", "+proj=unitconvert", "+xy_in=rad", "+xy_out=deg"],
                       input=f"{east} {north} 0 0\n", check=True,
                       capture_output=True, text=True).stdout.split()
    return float(o[0]), float(o[1])


# AdV's printed records: (point, Gauss-Krüger zone, easting, northing, UTM zone,
# UTM easting, UTM northing).
ADV = [
    ("point 1 record 2", 2, 2598417.333192, 5930677.980308, 32, 399340.601863, 5928794.177992),
    ("point 1 record 3", 3, 3399371.190396, 5930724.531323, 32, 399340.601862, 5928794.177992),
    ("point 2 record 2", 3, 3599586.686397, 5819391.659845, 32, 599474.934168, 5817502.626999),
    ("point 2 record 3", 4, 4395886.918912, 5819485.694352, 32, 599474.934169, 5817502.626999),
]

# Places across Germany, as DHDN longitude and latitude; the Gauss-Krüger zone
# is the nearest three-degree meridian, as the tool picks it.
PLACES = [
    ("Berlin, the Brandenburg Gate", 13.377778, 52.516389),
    ("Hamburg, the Rathaus", 9.992222, 53.550833),
    ("Munich, the Frauenkirche", 11.573611, 48.138611),
    ("Cologne cathedral", 6.958056, 50.941389),
    ("Frankfurt, the Römer", 8.681389, 50.110556),
    ("Stuttgart, the Schlossplatz", 9.177778, 48.778333),
    ("Dresden, the Frauenkirche", 13.741667, 51.051944),
    ("Leipzig, the Völkerschlachtdenkmal", 12.413333, 51.312222),
    ("Bremen, the Roland", 8.807222, 53.075833),
    ("Hanover, the Neues Rathaus", 9.735278, 52.366667),
    ("Nuremberg castle", 11.075556, 49.457778),
    ("Freiburg cathedral", 7.852500, 47.995556),
    ("Rostock, the Petrikirche", 12.143333, 54.088889),
    ("Saarbrücken, the Schloss", 6.993889, 49.231667),
    ("Flensburg", 9.436944, 54.782222),
    ("Passau, the Veste Oberhaus", 13.468889, 48.577778),
    ("Kiel, the Rathaus", 10.135278, 54.321389),
    ("Magdeburg cathedral", 11.635278, 52.123889),
    ("Erfurt, the Domplatz", 11.023611, 50.976111),
    ("Konstanz, the Münster", 9.176111, 47.663611),
    ("Aachen cathedral", 6.084167, 50.774722),
    ("Schwerin castle", 11.418056, 53.623611),
]


def gk_zone_for(lon):
    return min(5, max(2, round(lon / 3)))


def dump(rows):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False,
                                separators=(",", ":"))
                     for i, r in enumerate(rows, 1)) + "\n"


def main():
    fwd, inv = [], []
    for name, gz, e, n, uz, eu, nu in ADV:
        fwd.append({
            "input": {"easting": f"{e} m", "northing": f"{n} m"},
            "expect": {"ok": True, "result.easting.value": eu, "result.northing.value": nu,
                       "result.zone": uz},
            "source": f"AdV, BeTA2007 documentation v1.5, section 4.2, {name}: DE_DHDN_3GK{gz} to ETRS89_UTM{uz}",
            "sourceVersion": "BeTA2007 documentation v1.5; grid BETA2007.gsb of 2009-11-06",
            "tolerance": {"result.zone": {"abs": 0},
                          "result.easting.value": {"abs": 1e-5},
                          "result.northing.value": {"abs": 1e-5}},
        })
        inv.append({
            "input": {"easting": f"{eu} m", "northing": f"{nu} m", "utm_zone": uz,
                      "gk_zone": gz},
            "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                       "result.zone": gz},
            "source": f"AdV, BeTA2007 documentation v1.5, section 4.2, {name}, read the other way: ETRS89_UTM{uz} to DE_DHDN_3GK{gz}",
            "sourceVersion": "BeTA2007 documentation v1.5; grid BETA2007.gsb of 2009-11-06",
            "tolerance": {"result.zone": {"abs": 0},
                          "result.easting.value": {"abs": 1e-4},
                          "result.northing.value": {"abs": 1e-4}},
        })

    for name, lon, lat in PLACES:
        gz = gk_zone_for(lon)
        # The place on its Gauss-Krüger zone, by PROJ, to put in.
        o = subprocess.run(["cct", "-d", "6", "+proj=pipeline",
                            "+step", "+proj=unitconvert", "+xy_in=deg", "+xy_out=rad",
                            "+step", *gk(gz)],
                           input=f"{lon} {lat} 0 0\n", check=True,
                           capture_output=True, text=True).stdout.split()
        ge, gn = round(float(o[0]), 4), round(float(o[1]), 4)
        uz = 32 if lon < 12 else 33
        eu, nu = to_utm(gz, ge, gn, uz)
        fwd.append({
            "input": {"easting": f"{ge} m", "northing": f"{gn} m"},
            "expect": {"ok": True, "result.easting.value": eu, "result.northing.value": nu,
                       "result.zone": uz},
            "source": f"PROJ 9.9.0 hgridshift applying de_adv_BETA2007.tif between EPSG 3146{gz + 3} and EPSG 258{uz}: {name}",
            "sourceVersion": "PROJ 9.9.0; AdV BETA2007.gsb of 2009-11-06",
            "tolerance": {"result.zone": {"abs": 0},
                          "result.easting.value": {"abs": 1e-5},
                          "result.northing.value": {"abs": 1e-5}},
        })
        a, b = to_gk(uz, round(eu, 4), round(nu, 4), gz)
        inv.append({
            "input": {"easting": f"{round(eu, 4)} m", "northing": f"{round(nu, 4)} m",
                      "utm_zone": uz},
            "expect": {"ok": True, "result.easting.value": a, "result.northing.value": b,
                       "result.zone": gz},
            "source": f"PROJ 9.9.0 hgridshift in reverse applying the same grid: {name}, ETRS89 UTM {uz} to Gauss-Krüger zone {gz}",
            "sourceVersion": "PROJ 9.9.0; AdV BETA2007.gsb of 2009-11-06",
            "tolerance": {"result.zone": {"abs": 0},
                          "result.easting.value": {"abs": 1e-5},
                          "result.northing.value": {"abs": 1e-5}},
        })

    # An easting with no zone in front of it, and one outside the grid.
    fwd.append({
        "input": {"easting": "399340.602 m", "northing": "5928794.178 m"},
        "expect": {"ok": False, "error.code": "INVALID_INPUT"},
        "source": "A German Gauss-Krüger easting carries its zone in front of it; a bare easting under 1,000,000 m names no zone",
        "sourceVersion": "BeTA2007 documentation v1.5, section 4.2",
        "tolerance": {},
    })
    fwd.append({
        "input": {"easting": "2300000 m", "northing": "5100000 m"},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": "BeTA2007 covers Germany, 5°30′E to 15°40′E and 47°N to 55°18′N; 300 km west of zone 2's meridian is in France, outside it",
        "sourceVersion": "AdV BETA2007.gsb of 2009-11-06",
        "tolerance": {},
    })
    inv.append({
        "input": {"easting": "500000 m", "northing": "4000000 m", "utm_zone": 32},
        "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
        "source": "On UTM zone 32, a northing of 4,000,000 m is in the Mediterranean, far south of the grid",
        "sourceVersion": "AdV BETA2007.gsb of 2009-11-06",
        "tolerance": {},
    })

    with open(sys.argv[2], "w") as f:
        f.write(dump(fwd))
    with open(sys.argv[3], "w") as f:
        f.write(dump(inv))


if __name__ == "__main__":
    main()
