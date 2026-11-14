#!/usr/bin/env python3
"""Golden vectors for geodesy.projection.mga-forward and mga-inverse
(add-international-grids, geodesy/national-grids).

Two oracles, because one of them cannot see the errors the other hides:

  * PROJ 9.9.0 through cs2cs applying each MGA zone's own EPSG record, so the
    projection's parameters come from the EPSG dataset rather than from this
    repository. These rows carry the coordinates at a millimetre.
  * GeographicLib's TransverseMercatorProj, which prints the convergence and
    the point scale factor beside the coordinates, and pins the coordinates a
    thousand times tighter. Building NZTM2000 found an ITM convergence 57.3
    times too large and a silent fall back to the wrong ellipsoid, both of
    which a millimetre tolerance on the coordinates alone let through.

    gen_mga.py FORWARD.jsonl INVERSE.jsonl

Writes nothing without paths: published files stay frozen and new vectors are
appended by hand.
"""
import json
import subprocess
import sys

GRS80_A = "6378137"
GRS80_RF = repr(1 / 298.257222101)

# EPSG's MGA zone CRSs declare their axes east then north already, but the
# grids are taken through the PROJ.4 rendering of the same EPSG record for the
# same reason gen_nztm.py does: it is unambiguous, and check_against_epsg()
# below refuses to run if projinfo stops giving these parameters for the code.
def mga_proj4(zone):
    return (f"+proj=utm +zone={zone} +south +ellps=GRS80 +units=m +no_defs")


GDA = "+proj=longlat +ellps=GRS80 +no_defs"


def central_meridian(zone):
    return 6 * zone - 183


def check_against_epsg():
    """Every zone's PROJ.4 string above must be that zone's EPSG record."""
    for zone in range(46, 60):
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{7800 + zone}"],
                              check=True, capture_output=True, text=True).stdout
        for p in ("+proj=utm", f"+zone={zone}", "+south", "+ellps=GRS80"):
            assert p in want, f"projinfo no longer gives {p} for EPSG:{7800 + zone}: {want}"
    # MGA94 is the same projection on GDA94, registered only for zones 48-58.
    for zone in range(48, 59):
        want = subprocess.run(["projinfo", "-o", "PROJ", f"EPSG:{28300 + zone}"],
                              check=True, capture_output=True, text=True).stdout
        assert f"+zone={zone}" in want and "+ellps=GRS80" in want, \
            f"EPSG:{28300 + zone} is no longer MGA zone {zone} on GRS 80: {want}"


def cs2cs(src, dst, x, y):
    out = subprocess.run(["cs2cs", "-d", "10", src, "+to", dst], input=f"{x} {y}\n",
                         check=True, capture_output=True, text=True).stdout.split()
    return float(out[0]), float(out[1])


def tmproj(lon0, lat, lon):
    """Easting offset, northing, gamma in degrees, and k, from GeographicLib."""
    out = subprocess.run(
        ["TransverseMercatorProj", "-l", str(lon0), "-k", "0.9996",
         "-e", GRS80_A, GRS80_RF, "-p", "9"],
        input=f"{lat} {lon}\n", check=True, capture_output=True, text=True,
    ).stdout.split()
    return float(out[0]), float(out[1]), float(out[2]), float(out[3])


# Places across every MGA zone EPSG registers, 46 to 59.
PLACES = [
    ("The Sydney Opera House", -33.858611, 151.214167),
    ("Melbourne, Flinders Street station", -37.818078, 144.967105),
    ("Brisbane, City Hall", -27.468968, 153.023499),
    ("Perth, Kings Park", -31.959500, 115.843900),
    ("Adelaide, Victoria Square", -34.928600, 138.600100),
    ("Hobart, Salamanca Place", -42.884400, 147.331000),
    ("Canberra, Parliament House", -35.307900, 149.124400),
    ("Darwin, the Esplanade", -12.463400, 130.845600),
    ("Alice Springs", -23.698000, 133.880700),
    ("Broome", -17.961400, 122.235900),
    ("Cairns", -16.918600, 145.778100),
    ("Christmas Island, Flying Fish Cove", -10.422200, 105.679400),
    ("Cocos (Keeling) Islands, West Island", -12.187800, 96.829100),
    ("Lord Howe Island", -31.555600, 159.082200),
    ("Norfolk Island, Kingston", -29.054600, 167.961900),
    ("The Indian Ocean west of the Cocos Islands, in zone 46", -12.000000, 94.500000),
    ("The Tasman Sea east of Norfolk Island, in zone 59", -29.000000, 170.000000),
]


def zone_for(lon):
    return int(((lon + 180) % 360) // 6) + 1


def rows():
    fwd, inv = [], []
    for name, lat, lon in PLACES:
        zone = zone_for(lon)
        assert 46 <= zone <= 59, f"{name} falls in zone {zone}"
        e, n = cs2cs(GDA, mga_proj4(zone), lon, lat)
        fwd.append({
            "input": {"lat": lat, "lon": lon},
            "expect": {"ok": True, "result.zone": zone,
                       "result.easting.value": e, "result.northing.value": n},
            "source": f"PROJ 9.9.0 cs2cs applying EPSG:{7800 + zone} (GDA2020 / MGA zone {zone}): {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-3},
                          "result.northing.value": {"abs": 1e-3}},
        })
        blon, blat = cs2cs(mga_proj4(zone), GDA, e, n)
        inv.append({
            "input": {"zone": zone, "easting": f"{e:.6f} m", "northing": f"{n:.6f} m"},
            "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
            "source": f"PROJ 9.9.0 cs2cs applying EPSG:{7800 + zone} in reverse: {name}",
            "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
            "tolerance": {"result.lat.value": {"abs": 1e-9},
                          "result.lon.value": {"abs": 1e-9}},
        })

    # The convergence and the point scale factor, from the second oracle, with
    # the coordinates pinned to the micrometre. One point sits on its zone's
    # central meridian, where the convergence is zero and the scale factor is
    # exactly the declared 0.9996.
    ANGLES = [
        ("The Sydney Opera House", -33.858611, 151.214167, 56),
        ("Perth, Kings Park", -31.959500, 115.843900, 50),
        ("Darwin, the Esplanade", -12.463400, 130.845600, 52),
        ("Hobart, Salamanca Place", -42.884400, 147.331000, 55),
        ("On zone 55's central meridian south of Melbourne", -38.000000, 147.000000, 55),
    ]
    tol = {"result.convergence.value": {"abs": 1e-7}, "result.scale_factor": {"abs": 1e-11}}
    for name, lat, lon, zone in ANGLES:
        cm = central_meridian(zone)
        x, y, gam, k = tmproj(cm, lat, lon)
        e, n = 500_000.0 + x, 10_000_000.0 + y
        fwd.append({
            "input": {"lat": lat, "lon": lon, "zone": zone},
            "expect": {"ok": True, "result.zone": zone, "result.easting.value": e,
                       "result.northing.value": n, "result.convergence.value": gam,
                       "result.scale_factor": k,
                       "result.central_meridian.value": float(cm)},
            "source": f"GeographicLib TransverseMercatorProj on MGA zone {zone}'s parameters: the grid coordinates, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**tol, "result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-6},
                          "result.northing.value": {"abs": 1e-6},
                          "result.central_meridian.value": {"abs": 1e-12}},
        })
        inv.append({
            "input": {"zone": zone, "easting": f"{e:.9f} m", "northing": f"{n:.9f} m"},
            "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon,
                       "result.convergence.value": gam, "result.scale_factor": k,
                       "result.central_meridian.value": float(cm)},
            "source": f"GeographicLib TransverseMercatorProj on MGA zone {zone}'s parameters, read back: the place, the convergence, and the point scale factor at {name}",
            "sourceVersion": "GeographicLib TransverseMercatorProj 2.7",
            "tolerance": {**tol, "result.lat.value": {"abs": 1e-11},
                          "result.lon.value": {"abs": 1e-11},
                          "result.central_meridian.value": {"abs": 1e-12}},
        })

    # The zones' own origins, which pin the declared parameters rather than a
    # place: on the central meridian at the equator the coordinates must be the
    # false origin exactly, and the scale factor the declared 0.9996.
    for zone in (46, 56, 59):
        cm = central_meridian(zone)
        fwd.append({
            "input": {"lat": 0, "lon": cm, "zone": zone},
            "expect": {"ok": True, "result.zone": zone, "result.easting.value": 500000.0,
                       "result.northing.value": 10000000.0, "result.scale_factor": 0.9996,
                       "result.convergence.value": 0.0,
                       "result.central_meridian.value": float(cm)},
            "source": f"EPSG:{7800 + zone}'s own origin: on MGA zone {zone}'s central meridian at the latitude of origin the easting is the false easting, the northing the false northing, the convergence zero, and the scale factor the declared 0.9996",
            "sourceVersion": "EPSG v13.102",
            "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-6},
                          "result.northing.value": {"abs": 1e-6},
                          "result.scale_factor": {"abs": 1e-12},
                          "result.convergence.value": {"abs": 1e-12},
                          "result.central_meridian.value": {"abs": 1e-12}},
        })

    # MGA94 is the same projection: the same latitude and longitude must give
    # the same easting and northing under either name. This is what the tool
    # claims in words, so a vector says it in numbers.
    name, lat, lon = "The Sydney Opera House", -33.858611, 151.214167
    e, n = cs2cs(GDA, mga_proj4(56), lon, lat)
    fwd.append({
        "input": {"lat": lat, "lon": lon, "datum": "MGA94"},
        "expect": {"ok": True, "result.zone": 56, "result.easting.value": e,
                   "result.northing.value": n,
                   "result.epsg": "EPSG:28356 (GDA94 / MGA zone 56)"},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:28356 (GDA94 / MGA zone 56), whose PROJ.4 rendering is identical to EPSG:7856's: {name}'s coordinates read as GDA94 land on the same easting and northing, because MGA94 and MGA2020 differ in datum and not in projection",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-3},
                      "result.northing.value": {"abs": 1e-3}},
    })
    inv.append({
        "input": {"zone": 56, "easting": f"{e:.6f} m", "northing": f"{n:.6f} m",
                  "datum": "MGA94"},
        "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon,
                   "result.epsg": "EPSG:28356 (GDA94 / MGA zone 56)"},
        "source": f"PROJ 9.9.0 cs2cs applying EPSG:28356 in reverse: {name}, with the answer on GDA94 rather than GDA2020",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.lat.value": {"abs": 1e-9},
                      "result.lon.value": {"abs": 1e-9}},
    })

    # A forced neighbouring zone: Brisbane is in zone 56, and a job that runs
    # west across the boundary may want it on zone 55. PROJ gives the answer
    # and the tool must warn that the zone is not the standard one.
    lat, lon = -27.468968, 153.023499
    e, n = cs2cs(GDA, mga_proj4(55), lon, lat)
    fwd.append({
        "input": {"lat": lat, "lon": lon, "zone": 55},
        "expect": {"ok": True, "result.zone": 55, "result.easting.value": e,
                   "result.northing.value": n,
                   "meta.warnings.*.code": "NONSTANDARD_ZONE"},
        "source": "PROJ 9.9.0 cs2cs applying EPSG:7855 (GDA2020 / MGA zone 55): Brisbane City Hall, which falls in zone 56, projected onto zone 55 as a job spanning the boundary would",
        "sourceVersion": "PROJ 9.9.0, EPSG v13.102",
        "tolerance": {"result.zone": {"abs": 0}, "result.easting.value": {"abs": 1e-3},
                      "result.northing.value": {"abs": 1e-3}},
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
