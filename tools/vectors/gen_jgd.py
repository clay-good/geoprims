#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.jgd2011-forward and jgd2011-inverse
(add-international-grids, geodesy/national-grids: "the 19 JGD2011 plane
rectangular zones as projections only").

Two oracles, as gen_mga.py uses, for the same reason:

  * PROJ 9.9.0 through cs2cs applying each system's own EPSG record (6669 to
    6687). EPSG declares these axes X (north) then Y (east), and cs2cs keeps
    that order, so the first number out is the northing. These rows carry the
    coordinates at a millimetre.
  * GeographicLib's TransverseMercatorProj, which prints the convergence and
    the point scale factor beside the coordinates. Its northing counts from
    the equator, so the origin's own northing is subtracted, and the rows pin
    the coordinates to the micrometre.

    gen_jgd.py FORWARD.jsonl INVERSE.jsonl

Writes nothing without paths: published files stay frozen and new vectors are
appended by hand.
"""
import json
import subprocess
import sys
from fractions import Fraction

GRS80_A = "6378137"
GRS80_RF = repr(1 / 298.257222101)
ROMAN = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII",
         "XIII", "XIV", "XV", "XVI", "XVII", "XVIII", "XIX"]

# MLIT Notice No. 9 of 2002, as GSI publishes it: (latitude, degrees and
# minutes of longitude) of each system's origin, system I first.
ORIGINS = [(33, 129, 30), (33, 131, 0), (36, 132, 10), (33, 133, 30), (36, 134, 20),
           (36, 136, 0), (36, 137, 10), (36, 138, 30), (36, 139, 50), (40, 140, 50),
           (44, 140, 15), (44, 142, 15), (44, 144, 15), (26, 142, 0), (26, 127, 30),
           (26, 124, 0), (26, 131, 0), (20, 136, 0), (26, 154, 0)]


def origin(zone):
    lat, d, m = ORIGINS[zone - 1]
    return float(lat), float(Fraction(d) + Fraction(m, 60))


def epsg(zone):
    return 6668 + zone


def check_against_epsg():
    """Every system's origin and scale above must be its EPSG record's."""
    for zone in range(1, 20):
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{epsg(zone)}"],
                              check=True, capture_output=True, text=True).stdout
        lat0, lon0 = origin(zone)
        got = dict(p.lstrip("+").split("=", 1) for p in want.split() if "=" in p)
        assert got["proj"] == "tmerc" and got["ellps"] == "GRS80", want
        assert float(got["k"]) == 0.9999 and float(got["x_0"]) == 0 and float(got["y_0"]) == 0, want
        assert float(got["lat_0"]) == lat0, (zone, want)
        assert abs(float(got["lon_0"]) - lon0) < 1e-11, (zone, want)
        name = subprocess.run(["projinfo", "-q", "-o", "WKT2:2019", f"EPSG:{epsg(zone)}"],
                              check=True, capture_output=True, text=True).stdout
        assert f'Japan Plane Rectangular CS {ROMAN[zone - 1]}"' in name, (zone, name[:120])


def cs2cs_fwd(zone, lat, lon):
    out = subprocess.run(["cs2cs", "-d", "10", "EPSG:6668", "+to", f"EPSG:{epsg(zone)}"],
                         input=f"{lat} {lon}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[1]), float(out[0])  # easting (Y), northing (X)


def cs2cs_inv(zone, e, n):
    out = subprocess.run(["cs2cs", "-d", "12", f"EPSG:{epsg(zone)}", "+to", "EPSG:6668"],
                         input=f"{n} {e}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def tmproj(zone, lat, lon):
    """Y, X, gamma in degrees, and k, from GeographicLib, X from the origin."""
    lat0, lon0 = origin(zone)

    def run(p, l):
        out = subprocess.run(
            ["TransverseMercatorProj", "-l", repr(lon0), "-k", "0.9999",
             "-e", GRS80_A, GRS80_RF, "-p", "9"],
            input=f"{p} {l}\n", check=True, capture_output=True, text=True,
        ).stdout.split()
        return [float(v) for v in out[:4]]

    _, y0, _, _ = run(lat0, lon0)
    x, y, gam, k = run(lat, lon)
    return x, y - y0, gam, k


# One place in every system, named with the prefecture the notice puts it in.
PLACES = [
    (1, "Nagasaki, Nagasaki Prefecture", 32.750286, 129.877667),
    (1, "Naze, Amami Oshima, Kagoshima Prefecture's islands", 28.377000, 129.493600),
    (2, "Fukuoka, Tenjin", 33.590200, 130.401700),
    (3, "Hiroshima, the Peace Memorial", 34.395500, 132.453600),
    (4, "Matsuyama Castle, Ehime", 33.845600, 132.765600),
    (5, "Kobe, Hyogo", 34.690100, 135.195500),
    (6, "Osaka Castle", 34.687315, 135.526201),
    (6, "Kyoto Station", 34.985849, 135.758767),
    (7, "Nagoya Castle, Aichi", 35.185600, 136.899200),
    (8, "Niigata Station", 37.911800, 139.061300),
    (9, "Tokyo Station", 35.681236, 139.767125),
    (9, "Yokohama, Kanagawa", 35.443700, 139.638000),
    (10, "Sendai, Miyagi", 38.268200, 140.869400),
    (11, "Hakodate, Oshima Subprefecture", 41.768793, 140.728810),
    (12, "Sapporo Clock Tower", 43.062562, 141.353650),
    (13, "Kushiro", 42.984854, 144.381356),
    (14, "Chichijima, Ogasawara", 27.094400, 142.191900),
    (15, "Naha, Shuri Castle", 26.217049, 127.719439),
    (16, "Ishigaki", 24.340661, 124.155652),
    (17, "Minamidaito", 25.845000, 131.246000),
    (18, "Okinotorishima", 20.425300, 136.080600),
    (19, "Minamitorishima", 24.286700, 153.980700),
]


def rows():
    fwd, inv = [], []
    for zone, name, lat, lon in PLACES:
        e, n = cs2cs_fwd(zone, lat, lon)
        src = f"EPSG:{epsg(zone)} (JGD2011 / Japan Plane Rectangular CS {ROMAN[zone - 1]})"
        fwd.append({
            "input": {"lat": lat, "lon": lon, "zone": zone},
            "expect": {"ok": True, "result.zone": zone,
                       "result.easting.value": e, "result.northing.value": n},
            "source": f"PROJ 9.9.0 cs2cs applying {src}: {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-3},
                          "result.northing.value": {"abs": 1e-3}},
        })
        blat, blon = cs2cs_inv(zone, e, n)
        inv.append({
            "input": {"zone": zone, "northing": f"{n:.6f} m", "easting": f"{e:.6f} m"},
            "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
            "source": f"PROJ 9.9.0 cs2cs applying {src} in reverse: {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.lat.value": {"abs": 1e-9},
                          "result.lon.value": {"abs": 1e-9}},
        })

    # The convergence and the point scale factor from the second oracle, with
    # the coordinates to the micrometre, including a far corner of a system.
    ANGLES = [
        (9, "Tokyo Station", 35.681236, 139.767125),
        (12, "Sapporo Clock Tower", 43.062562, 141.353650),
        (1, "Naze, Amami Oshima", 28.377000, 129.493600),
        (19, "Minamitorishima", 24.286700, 153.980700),
        (13, "Nemuro, at the east edge of system XIII", 43.330000, 145.583000),
    ]
    tol = {"result.convergence.value": {"abs": 1e-7}, "result.scale_factor": {"abs": 1e-11}}
    for zone, name, lat, lon in ANGLES:
        lat0, lon0 = origin(zone)
        e, n, gam, k = tmproj(zone, lat, lon)
        common = {"result.convergence.value": gam, "result.scale_factor": k,
                  "result.origin_lat.value": lat0, "result.central_meridian.value": lon0}
        ctol = {**tol, "result.origin_lat.value": {"abs": 1e-12},
                "result.central_meridian.value": {"abs": 1e-12}}
        fwd.append({
            "input": {"lat": lat, "lon": lon, "zone": zone},
            "expect": {"ok": True, "result.zone": zone, "result.easting.value": e,
                       "result.northing.value": n, **common},
            "source": f"GeographicLib TransverseMercatorProj on system {ROMAN[zone - 1]}'s parameters, the origin's own northing subtracted: X, Y, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**ctol, "result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-6},
                          "result.northing.value": {"abs": 1e-6}},
        })
        inv.append({
            "input": {"zone": zone, "northing": f"{n:.9f} m", "easting": f"{e:.9f} m"},
            "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon, **common},
            "source": f"GeographicLib TransverseMercatorProj on system {ROMAN[zone - 1]}'s parameters, read back: the place, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**ctol, "result.lat.value": {"abs": 1e-11},
                          "result.lon.value": {"abs": 1e-11}},
        })

    # The origins pin the notice's parameters rather than a place: there X and
    # Y are zero, the convergence zero, and the scale factor the declared 0.9999.
    for zone in (1, 9, 10, 18):
        lat0, lon0 = origin(zone)
        fwd.append({
            "input": {"lat": lat0, "lon": lon0, "zone": zone},
            "expect": {"ok": True, "result.zone": zone, "result.easting.value": 0.0,
                       "result.northing.value": 0.0, "result.scale_factor": 0.9999,
                       "result.convergence.value": 0.0,
                       "result.origin_lat.value": lat0,
                       "result.central_meridian.value": lon0,
                       "result.epsg": f"EPSG:{epsg(zone)} (JGD2011 / Japan Plane Rectangular CS {ROMAN[zone - 1]})"},
            "source": f"MLIT Notice No. 9 of 2002 for system {ROMAN[zone - 1]}: at the origin X = 0.000 m and Y = 0.000 m, and the scale factor on the X axis is 0.9999",
            "sourceVersion": "MLIT Notice No. 9 of 2002, last amended 2010",
            "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-6},
                          "result.northing.value": {"abs": 1e-6},
                          "result.scale_factor": {"abs": 1e-12},
                          "result.convergence.value": {"abs": 1e-12},
                          "result.origin_lat.value": {"abs": 1e-12},
                          "result.central_meridian.value": {"abs": 1e-12}},
        })

    # Off Japan altogether: the arithmetic holds and the answer says where it is.
    lat, lon = 51.5074, -0.1278
    e, n = cs2cs_fwd(9, lat, lon)
    fwd.append({
        "input": {"lat": lat, "lon": lon, "zone": 9},
        "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                   "meta.warnings.*.code": "OUTSIDE_AREA_OF_USE"},
        "source": "PROJ 9.9.0 cs2cs applying EPSG:6677: London on system IX, far outside JGD2011's area of use, which the tool must flag",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}},
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
        sys.stdout.write(dump(fwd))
        sys.stdout.write("--- inverse ---\n")
        sys.stdout.write(dump(inv))
        return
    with open(sys.argv[1], "w") as f:
        f.write(dump(fwd))
    with open(sys.argv[2], "w") as f:
        f.write(dump(inv))


if __name__ == "__main__":
    main()
