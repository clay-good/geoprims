#!/usr/bin/env python3
"""Golden vectors for the national-grid conversion endpoints
(add-international-grids task 7): geodesy.grid-ref.latlon-to-bng and
bng-to-latlon, geodesy.projection.mga-to-latlon and nztm-to-latlon.

Each endpoint is its parent with the datum fixed, so its vectors come from the
same oracle as the parent's, through the same definitions: the National Grid
and EPSG 1314's datum step from gen_bng.py, the MGA zones from gen_mga.py, and
NZTM2000 from gen_nztm.py, all imported rather than written again, so the two
sets of vectors cannot drift apart. Each file's first row is the endpoint's own
primary example. No input names the preset field: the endpoint fixes it.

    gen_grid_pairs.py           # print the rows, grouped by file
    gen_grid_pairs.py DIR       # write DIR/<tool>.jsonl

Published files are frozen: once pushed, new rows are appended by hand.
"""
import contextlib
import io
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
with contextlib.redirect_stdout(io.StringIO()):
    import gen_bng  # noqa: E402  (prints its own rows on import)
import gen_mga  # noqa: E402
import gen_nztm  # noqa: E402

PROJ = "PROJ 9.9.0 cs2cs"
VERSION = "PROJ 9.9.0, EPSG v13.102"
MM = {"abs": 1e-3}
NANODEG = {"abs": 1e-9}


def latlon_to_bng():
    """WGS 84 positions to references, as the parent computes them on WGS 84."""
    rows = []
    places = [("The Tower of London", 51.508056, -0.076111, 10)] + [
        (name, lat, lon, fig) for name, lat, lon, fig in gen_bng.PLACES[:5]
    ]
    for name, lat, lon, figures in places:
        e, n = gen_bng.cs2cs("EPSG:4326", gen_bng.BNG_FROM_WGS, lat, lon)
        rows.append({
            "input": {"lat": lat, "lon": lon, "figures": figures},
            "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                       "result.reference": gen_bng.reference(e, n, figures)},
            "source": f"{PROJ} with the National Grid and EPSG 1314 as gen_bng.py writes them: {name}, from WGS 84",
            "sourceVersion": VERSION,
            "tolerance": {"result.easting.value": MM, "result.northing.value": MM},
        })
    return rows


def bng_to_latlon():
    """References to the WGS 84 position of the square's center."""
    rows = []
    reads = [
        ("SU 1224 4219", "Stonehenge's square", 412240.0, 142190.0, 10.0),
        ("TQ 30 80", "Westminster's square", 530000.0, 180000.0, 1000.0),
        ("NT 2746 7355", "Edinburgh's Old Town", 327460.0, 673550.0, 10.0),
        ("SH 60 54", "Snowdon's square", 260000.0, 354000.0, 1000.0),
        ("TG 51409 13177", "the guide's Annexe C point", 651409.0, 313177.0, 1.0),
    ]
    for ref, name, e, n, side in reads:
        lat, lon = gen_bng.cs2cs(gen_bng.BNG_FROM_WGS, "EPSG:4326", e + side / 2, n + side / 2)
        rows.append({
            "input": {"reference": ref},
            "expect": {"ok": True, "result.easting.value": e, "result.northing.value": n,
                       "result.square_size.value": side,
                       "result.lat.value": lat, "result.lon.value": lon},
            "source": f"{PROJ} with the National Grid and EPSG 1314 as gen_bng.py writes them, in reverse: the center of {ref}, {name}, on WGS 84",
            "sourceVersion": VERSION,
            "tolerance": {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
                          "result.square_size.value": {"abs": 1e-6},
                          "result.lat.value": NANODEG, "result.lon.value": NANODEG},
        })
    return rows


def mga_to_latlon():
    """MGA2020 coordinates back to GDA2020, on each zone's EPSG record."""
    rows = []
    # The first row is the endpoint's example exactly as pairs.rs writes it:
    # Uluru (25.3444 S 131.0369 E) on zone 52, to the millimetre the page shows.
    grid = [("Uluru", 52, "704993.882", "7195356.236")]
    for name, lat, lon in gen_mga.PLACES[:5]:
        zone = gen_mga.zone_for(lon)
        e, n = gen_mga.cs2cs(gen_mga.GDA, gen_mga.mga_proj4(zone), lon, lat)
        grid.append((name, zone, repr(e), repr(n)))
    for name, zone, e, n in grid:
        blon, blat = gen_mga.cs2cs(gen_mga.mga_proj4(zone), gen_mga.GDA, e, n)
        rows.append({
            "input": {"zone": zone, "easting": f"{e} m", "northing": f"{n} m"},
            "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
            "source": f"{PROJ} applying EPSG:{7800 + zone} (GDA2020 / MGA zone {zone}) in reverse: {name}",
            "sourceVersion": VERSION,
            "tolerance": {"result.lat.value": NANODEG, "result.lon.value": NANODEG},
        })
    return rows


def nztm_to_latlon():
    """NZTM2000 coordinates back to NZGD2000, on EPSG 2193."""
    rows = []
    # The first row is the endpoint's example exactly as pairs.rs writes it:
    # Queenstown (45.0312 S 168.6626 E), to the millimetre the page shows.
    grid = [("Queenstown", "1258332.205", "5004425.269")]
    for name, lat, lon in gen_nztm.PLACES[:5]:
        e, n = gen_nztm.cs2cs(gen_nztm.NZGD, gen_nztm.NZTM, lon, lat)
        grid.append((name, repr(e), repr(n)))
    for name, e, n in grid:
        blon, blat = gen_nztm.cs2cs(gen_nztm.NZTM, gen_nztm.NZGD, e, n)
        rows.append({
            "input": {"easting": f"{e} m", "northing": f"{n} m"},
            "expect": {"ok": True, "result.lat.value": blat, "result.lon.value": blon},
            "source": f"{PROJ} applying EPSG:2193 in reverse: {name}",
            "sourceVersion": VERSION,
            "tolerance": {"result.lat.value": NANODEG, "result.lon.value": NANODEG},
        })
    return rows


FILES = {
    "geodesy.grid-ref.latlon-to-bng": latlon_to_bng,
    "geodesy.grid-ref.bng-to-latlon": bng_to_latlon,
    "geodesy.projection.mga-to-latlon": mga_to_latlon,
    "geodesy.projection.nztm-to-latlon": nztm_to_latlon,
}


def dump(rows):
    return "\n".join(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":"))
                     for i, r in enumerate(rows, 1)) + "\n"


def main():
    gen_mga.check_against_epsg()
    for tool, build in FILES.items():
        text = dump(build())
        if len(sys.argv) > 1:
            with open(os.path.join(sys.argv[1], f"{tool}.jsonl"), "w") as f:
                f.write(text)
        else:
            sys.stdout.write(f"--- {tool} ---\n{text}")


if __name__ == "__main__":
    main()
