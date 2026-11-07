#!/usr/bin/env python3
"""Convergence and point scale factor vectors for the national transverse
Mercator grids (add-international-grids, geodesy/national-grids).

These exist because nothing pinned them. The ITM tools shipped a grid
convergence 57.3 times too large -- the series returns gamma in degrees and the
tool converted it again -- and every gate stayed green: no vector carried a
convergence, and the invariant test only checked that the forward and inverse
tools agreed, which they did, both being wrong by the same factor.

The oracle is GeographicLib's TransverseMercatorProj, which prints gamma and k
beside the coordinates. It is a second reference, independent of the PROJ runs
that made the coordinate vectors in gen_itm.py and gen_nztm.py.

    gen_tm_convergence.py            # print the rows, grouped by file
    gen_tm_convergence.py DIR        # write <DIR>/<tool>.append.jsonl

The coordinate vectors in the published files are frozen; these rows are
appended after them by hand, continuing the numbering.
"""
import json
import subprocess
import sys

GRS80 = ("6378137", repr(1 / 298.257222101))

# tool stem, lat0, lon0, k0, false easting, false northing, and places on both
# sides of the central meridian, including one on it, where convergence is zero.
GRIDS = [
    (
        "geodesy.grid-ref.itm",
        53.5,
        -8.0,
        0.99982,
        600_000.0,
        750_000.0,
        "EPSG 2157 (Irish Transverse Mercator)",
        [
            ("The Spire of Dublin", 53.349806, -6.260247),
            ("Cork, Shandon Bells", 51.902100, -8.473400),
            ("Galway Cathedral", 53.275800, -9.057200),
            ("Malin Head", 55.381400, -7.372500),
            ("On the central meridian at Birr", 53.096000, -8.000000),
        ],
    ),
    (
        "geodesy.projection.nztm",
        0.0,
        173.0,
        0.9996,
        1_600_000.0,
        10_000_000.0,
        "EPSG 2193 (New Zealand Transverse Mercator 2000)",
        [
            ("The Beehive, Wellington", -41.278611, 174.776944),
            ("Auckland, the Sky Tower", -36.848461, 174.762191),
            ("Invercargill", -46.413100, 168.352800),
            ("East Cape", -37.688900, 178.548600),
            ("On the central meridian off Kaikoura", -42.000000, 173.000000),
        ],
    ),
]


def tmproj(lon0, k0, lat, lon):
    """Easting, northing, gamma in degrees, and k, from GeographicLib."""
    out = subprocess.run(
        ["TransverseMercatorProj", "-l", str(lon0), "-k", str(k0),
         "-e", GRS80[0], GRS80[1], "-p", "9"],
        input=f"{lat} {lon}\n", check=True, capture_output=True, text=True,
    ).stdout.split()
    return float(out[0]), float(out[1]), float(out[2]), float(out[3])


def rows():
    """Each file's appended rows, keyed by tool id."""
    out = {}
    for stem, lat0, lon0, k0, fe, fn, epsg, places in GRIDS:
        fwd, inv = [], []
        # The grid counts its northing from the latitude of origin, which the
        # projection puts at a northing of its own; take it out the same way
        # the core does, from the origin's own projection.
        _, y0, _, _ = tmproj(lon0, k0, lat0, lon0)
        for name, lat, lon in places:
            x, y, gam, k = tmproj(lon0, k0, lat, lon)
            e, n = fe + x, fn + y - y0
            angle = {"result.convergence.value": {"abs": 1e-7}, "result.scale_factor": {"abs": 1e-11}}
            # The coordinates are pinned a thousand times tighter than the
            # millimetre the PROJ rows allow, because projecting on the wrong
            # ellipsoid moved them by less than that millimetre and passed:
            # WGS 84 in place of GRS 80 is 1.2 um of ITM easting and 0.12 mm of
            # NZTM northing, which this catches and a 1 mm tolerance does not.
            fwd.append({
                "input": {"lat": lat, "lon": lon},
                "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                           "result.convergence.value": gam, "result.scale_factor": k},
                "source": f"GeographicLib TransverseMercatorProj on {epsg}'s parameters: the grid coordinates, the convergence, and the point scale factor at {name}",
                "sourceVersion": "GeographicLib TransverseMercatorProj, as installed",
                "tolerance": {**angle, "result.easting.value": {"abs": 1e-6},
                              "result.northing.value": {"abs": 1e-6}},
            })
            inv.append({
                "input": {"easting": f"{e:.9f} m", "northing": f"{n:.9f} m"},
                "expect": {"ok": True, "result.lat.value": lat, "result.lon.value": lon,
                           "result.convergence.value": gam, "result.scale_factor": k},
                "source": f"GeographicLib TransverseMercatorProj on {epsg}'s parameters, read back: the place, the convergence, and the point scale factor at {name}",
                "sourceVersion": "GeographicLib TransverseMercatorProj, as installed",
                "tolerance": {**angle, "result.lat.value": {"abs": 1e-11},
                              "result.lon.value": {"abs": 1e-11}},
            })
        out[f"{stem}-forward"] = fwd
        out[f"{stem}-inverse"] = inv
    return out


def main():
    all_rows = rows()
    for tool, rs in all_rows.items():
        text = "\n".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rs) + "\n"
        if len(sys.argv) > 1:
            with open(f"{sys.argv[1]}/{tool}.append.jsonl", "w") as f:
                f.write(text)
        else:
            sys.stdout.write(f"--- {tool} ---\n{text}")


if __name__ == "__main__":
    main()
