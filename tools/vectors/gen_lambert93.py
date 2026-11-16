#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.lambert93-forward and
lambert93-inverse (add-international-grids, geodesy/national-grids:
"RGF93 Lambert-93").

Two oracles:

  * PROJ 9.9.0 through cs2cs applying each grid's own EPSG record: 2154 for
    Lambert-93 and 3942 to 3950 for CC42 to CC50, from RGF93 v1 (4171). These
    rows carry the coordinates at a millimetre.
  * IGN's own constants for Lambert-93, as NT/G 71 prints them (n, c, Xs, Ys),
    put through NT/G 71's ALG0003 here in Python: the coordinates, and the
    convergence n(lambda - lambda0) and point scale factor n R / (N cos phi)
    from the same constants. The printed n carries ten decimals and c three,
    which agree with PROJ to 0.15 mm across France.

    gen_lambert93.py FORWARD.jsonl INVERSE.jsonl
"""
import json
import math
import subprocess
import sys

A = 6378137.0
F = 1 / 298.257222101
E2 = F * (2 - F)
E = math.sqrt(E2)
# NT/G 71, "Valeurs des constantes Lambert France", the Lambert-93 column.
IGN = {"n": 0.7256077650, "c": 11754255.426, "Xs": 700000.0, "Ys": 12655612.050}


def epsg(zone):
    return 2154 if zone == 0 else 3900 + zone


def name(zone):
    return "Lambert-93" if zone == 0 else f"CC{zone}"


def check_against_epsg():
    want = subprocess.run(["projinfo", "-o", "PROJ", "EPSG:2154"], check=True,
                          capture_output=True, text=True).stdout
    for p in ("+proj=lcc", "+lat_0=46.5", "+lon_0=3 ", "+lat_1=49", "+lat_2=44",
              "+x_0=700000", "+y_0=6600000", "+ellps=GRS80"):
        assert p in want, (p, want)
    for z in range(42, 51):
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{epsg(z)}"], check=True,
                              capture_output=True, text=True).stdout
        for p in (f"+lat_0={z} ", "+lon_0=3 ", f"+lat_1={z - 0.75:g} ", f"+lat_2={z + 0.75:g} ",
                  "+x_0=1700000", f"+y_0={(z - 41) * 1000000 + 200000} ", "+ellps=GRS80"):
            assert p in want, (z, p, want)


def cs2cs_fwd(zone, lat, lon):
    out = subprocess.run(["cs2cs", "-d", "10", "EPSG:4171", "+to", f"EPSG:{epsg(zone)}"],
                         input=f"{lat} {lon}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def cs2cs_inv(zone, e, n):
    out = subprocess.run(["cs2cs", "-d", "12", f"EPSG:{epsg(zone)}", "+to", "EPSG:4171"],
                         input=f"{e} {n}\n", check=True, capture_output=True,
                         text=True).stdout.split()
    return float(out[0]), float(out[1])


def ign(lat, lon):
    """NT/G 71 ALG0001 and ALG0003 with the printed Lambert-93 constants."""
    phi = math.radians(lat)
    s = math.sin(phi)
    iso = math.log(math.tan(math.pi / 4 + phi / 2) * ((1 - E * s) / (1 + E * s)) ** (E / 2))
    r = IGN["c"] * math.exp(-IGN["n"] * iso)
    th = IGN["n"] * math.radians(lon - 3.0)
    x = IGN["Xs"] + r * math.sin(th)
    y = IGN["Ys"] - r * math.cos(th)
    nu = A / math.sqrt(1 - E2 * s * s)
    return x, y, math.degrees(th), IGN["n"] * r / (nu * math.cos(phi))


PLACES = [
    ("The Eiffel Tower, Paris", 48.858370, 2.294481, 49),
    ("Lille, the Grand Place", 50.636800, 3.063500, 50),
    ("Strasbourg Cathedral", 48.581700, 7.750800, 49),
    ("Brest", 48.390400, -4.486100, 48),
    ("Nantes, the Château des ducs", 47.216000, -1.549500, 47),
    ("Lyon, Place Bellecour", 45.757800, 4.832000, 46),
    ("Clermont-Ferrand Cathedral", 45.778900, 3.085700, 46),
    ("Bordeaux, Place de la Bourse", 44.841600, -0.569700, 45),
    ("Toulouse, the Capitole", 43.604500, 1.443800, 44),
    ("Marseille, the Vieux-Port", 43.295100, 5.374000, 43),
    ("Nice, the Promenade des Anglais", 43.695000, 7.265000, 44),
    ("Perpignan", 42.698600, 2.895600, 43),
    ("Ajaccio, Corsica", 41.919200, 8.738600, 42),
    ("Bastia, Corsica", 42.697300, 9.450800, 43),
]


def rows():
    fwd, inv = [], []
    mm = {"result.easting.value": {"abs": 1e-3}, "result.northing.value": {"abs": 1e-3}}
    for place, lat, lon, cc in PLACES:
        for zone in (0, cc):
            e, n = cs2cs_fwd(zone, lat, lon)
            src = f"EPSG:{epsg(zone)} (RGF93 v1 / {name(zone)})"
            inp = {"lat": lat, "lon": lon}
            if zone:
                inp["grid"] = name(zone)
            fwd.append({
                "input": inp,
                "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                           "result.grid": name(zone)},
                "source": f"PROJ 9.9.0 cs2cs applying {src}: {place}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": mm,
            })
            blat, blon = cs2cs_inv(zone, e, n)
            ii = {"easting": f"{e:.6f} m", "northing": f"{n:.6f} m"}
            if zone:
                ii["grid"] = name(zone)
            inv.append({
                "input": ii,
                "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
                "source": f"PROJ 9.9.0 cs2cs applying {src} in reverse: {place}",
                "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
                "tolerance": {"result.lat.value": {"abs": 1e-9}, "result.lon.value": {"abs": 1e-9}},
            })

    # IGN's printed constants, through IGN's own algorithm, at five places.
    tol = {**mm, "result.convergence.value": {"abs": 1e-8},
           "result.scale_factor": {"abs": 1e-9}}
    for place, lat, lon in [("The Eiffel Tower, Paris", 48.858370, 2.294481),
                            ("Brest", 48.390400, -4.486100),
                            ("Strasbourg Cathedral", 48.581700, 7.750800),
                            ("Ajaccio, Corsica", 41.919200, 8.738600),
                            ("On the central meridian at the latitude of origin", 46.5, 3.0)]:
        x, y, gam, k = ign(lat, lon)
        fwd.append({
            "input": {"lat": lat, "lon": lon},
            "expect": {"ok": True, "result.easting.value": x, "result.northing.value": y,
                       "result.convergence.value": gam, "result.scale_factor": k},
            "source": f"IGN NT/G 71 (1995), ALG0003 with the printed Lambert-93 constants n 0.7256077650, c 11 754 255.426 m, Xs 700 000 m, Ys 12 655 612.050 m; the convergence n(lambda - lambda0) and the scale factor nR/(N cos phi) from the same constants: {place}",
            "sourceVersion": "IGN NT/G 71, 1st edition, January 1995",
            "tolerance": tol,
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
