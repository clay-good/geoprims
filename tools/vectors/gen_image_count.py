#!/usr/bin/env python3
"""Vectors for drone.photogrammetry.image-count and drone.mission.survey-grid
under the published flight-planning count (tool version 1.1.0).

    gen_image_count.py OUTDIR

Writes the vectors this script makes (not the frozen v001-v005 lines) to
OUTDIR/<tool id>.jsonl; append them to core/vectors by hand. Expected values
come from the published method evaluated here on the block's geodesic size,
never from the tool:

  lines  = ceil(width / line spacing) + 1
  photos = lines x (ceil(length / photo spacing) + 1 + 2 x extra per end)

Penn State GEOG 892, "Designing a Flight Route" (Q. Abdullah): 13 x 20 mi,
8,400 ft spacing, 2,800 ft air base -> 10 lines, 39 + 4 = 43 photos per line,
430 photos. King Saud University SE 321, "Design of Photogrammetric Flight
Plan" example: 40 x 60 km, 920 m spacing, 460 m air base -> 45 lines,
132 + 4 = 136 per line, 6,120 photos.

Each rectangle's width is the meridian arc between its latitudes and its
length the parallel arcs at its south and north edges (geographiclib). A
count is pinned only when every length the lines can take gives the same
ceiling with a margin of 1e-4, so the tool's plane (scale error under 1e-6
here) cannot flip it.
"""
import json
import math
import sys
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
FT = 0.3048
IC = "drone.photogrammetry.image-count"
SG = "drone.mission.survey-grid"
PSU = ("Penn State GEOG 892 (Q. Abdullah), Designing a Flight Route: (13 x 5,280 / 8,400) + 1 = 9.171, "
       "10 lines; (105,600 / 2,800) + 1 = 38.7, 39, + 4 = 43 images per line; 430 images")
PSU_V = "Penn State College of Earth and Mineral Sciences, online course text, retrieved 2026-09-23"
KSU = ("King Saud University SE 321, Design of Photogrammetric Flight Plan (CLO3 example): (60,000 / 460) + 1 = 131.4, "
       "132 + 4 = 136 photos per strip; (40,000 / 920) + 1 = 44.4, 45 lines; 136 x 45 = 6,120 photos")
KSU_V = "faculty.ksu.edu.sa, retrieved 2026-09-23"
METHOD = ("Published flight-line count (Penn State GEOG 892): ceil(width / spacing) + 1 lines, "
          "ceil(length / photo spacing) + 1 + 2 x extra photos per line, on geographiclib arc lengths (tools/vectors/gen_image_count.py)")
METHOD_V = "tool version 1.1.0"


def parse_len(s):
    v, u = s.split()
    return float(v) * {"m": 1.0, "ft": FT}[u]


def arcs(lat1, lat2, lon1, lon2):
    w = G.Inverse(lat1, lon1, lat2, lon1)["s12"]
    along = [parallel(lat, lon1, lon2) for lat in (lat1, lat2)]
    return w, along


def parallel(lat, lon1, lon2):
    # Arc length along the parallel: N(lat) cos(lat) dlon.
    a, f = G.a, G.f
    e2 = f * (2 - f)
    phi = math.radians(lat)
    n = a / math.sqrt(1 - e2 * math.sin(phi) ** 2)
    return n * math.cos(phi) * math.radians(abs(lon2 - lon1))


def ceil_safe(xs, what):
    """The ceiling every value in xs shares, far enough from a whole number to pin."""
    out = set()
    for x in xs:
        frac = x - math.floor(x)
        if min(frac, 1 - frac) < 1e-4:
            raise ValueError(f"{what}: {x} is too close to a whole number to pin")
        out.add(math.ceil(x))
    if len(out) != 1:
        raise ValueError(f"{what}: {xs} straddle a whole number")
    return out.pop()


def count(rect, spacing, photo, extra=2, direction=None):
    lat1, lat2, lon1, lon2 = rect
    w, along = arcs(lat1, lat2, lon1, lon2)
    # Lines run east-west when asked (90 deg) or, on auto, along the longer side.
    east_west = direction == "90 deg" or (direction is None and max(along) >= w)
    if east_west:
        widths, lengths = [w], along
    else:
        widths, lengths = along, [w]
    lines = ceil_safe([x / spacing for x in widths], "lines") + 1
    per = ceil_safe([x / photo for x in lengths], "photos") + 1 + 2 * extra
    return lines, lines * per, lines * sum(lengths) / len(lengths)


def area(rect):
    lat1, lat2, lon1, lon2 = rect
    return [{"lat": lat1, "lon": lon1}, {"lat": lat1, "lon": lon2}, {"lat": lat2, "lon": lon2}, {"lat": lat2, "lon": lon1}]


def ok(tool, vid, rect, spacing, photo, source, version, extra=None, direction=None, survey=False, pin_photos=True):
    inp = {"area": area(rect), "line_spacing": spacing, "photo_spacing": photo}
    if extra is not None:
        inp["end_photos"] = extra
    if direction is not None:
        inp["direction"] = direction
    lines, photos, length = count(rect, parse_len(spacing), parse_len(photo), 2 if extra is None else extra, direction)
    expect = {"result.lines": float(lines)}
    tol = {"result.lines": {"abs": 0}}
    if pin_photos:
        expect["result.photos"] = float(photos)
        tol["result.photos"] = {"abs": 0}
    if survey:
        expect["result.survey_length.value"] = round(length / 1000.0, 6)
        tol["result.survey_length.value"] = {"rel": 1e-5}
    expect["ok"] = True
    return {"id": vid, "input": inp, "expect": expect, "source": source, "sourceVersion": version, "tolerance": tol}


def err(vid, inp, code, why):
    return {"id": vid, "input": inp, "expect": {"ok": False, "error.code": code}, "source": f"{why} (tools/vectors/gen_image_count.py)", "sourceVersion": METHOD_V}


# The five frozen rectangles of v001-v005, recounted.
OLD = [
    ((40.0, 40.00135, -105.0, -104.99295), "52.5 m"),
    ((35.0, 35.009, -100.0, -99.998), "40.0 m"),
    ((51.0, 51.004, 0.1, 0.112), "60.0 m"),
    ((-33.0, -32.9978, 151.0, 151.0022), "25.0 m"),
    ((60.0, 60.02, 25.0, 25.005), "100.0 m"),
]
PSU_RECT = (-0.094604, 0.094604, -0.14457, 0.14457)
KSU_RECT = (-0.180874, 0.180874, -0.269495, 0.269495)
BASE = {"area": area((40.0, 40.00135, -105.0, -104.99295)), "line_spacing": "52.5 m", "photo_spacing": "30 m"}


def image_count():
    out = []
    vid = iter(f"v{i:03d}" for i in range(6, 100))
    for rect, sp in OLD:
        out.append(ok(IC, next(vid), rect, sp, "30 m", METHOD, METHOD_V))
    out.append(ok(IC, next(vid), PSU_RECT, "8400 ft", "2800 ft", PSU, PSU_V, survey=True))
    out.append(ok(IC, next(vid), PSU_RECT, "8400 ft", "2800 ft", PSU + " (390 without the extra images)", PSU_V, extra=0))
    out.append(ok(IC, next(vid), PSU_RECT, "8400 ft", "2800 ft", PSU, PSU_V, extra=2))
    out.append(ok(IC, next(vid), KSU_RECT, "920 m", "460 m", KSU, KSU_V, survey=True))
    out.append(ok(IC, next(vid), KSU_RECT, "920 m", "460 m", KSU + " (5,940 without the extra photos)", KSU_V, extra=0))
    # The Penn State block flown north-south: ceil(105,600 / 8,400) + 1 = 14 lines of 25 + 1 + 4 photos.
    out.append(ok(IC, next(vid), PSU_RECT, "8400 ft", "2800 ft", METHOD, METHOD_V, direction="0 deg"))
    out.append(ok(IC, next(vid), PSU_RECT, "8400 ft", "2800 ft", METHOD, METHOD_V, extra=1, survey=True))
    out.append(ok(IC, next(vid), KSU_RECT, "920 m", "460 m", METHOD, METHOD_V, extra=5))
    out.append(ok(IC, next(vid), KSU_RECT, "1500 m", "700 m", METHOD, METHOD_V, survey=True))
    out.append(ok(IC, next(vid), PSU_RECT, "30000 ft", "10000 ft", METHOD + ": spacings a third of the block, ceil(2.29) + 1 = 4 lines of ceil(10.56) + 1 + 4 = 16 photos", METHOD_V))
    out.append(err(next(vid), {**BASE, "end_photos": 1.5}, "INVALID_INPUT", "Extra photos per line end must be a whole number"))
    out.append(err(next(vid), {**BASE, "end_photos": 11}, "INVALID_INPUT", "Extra photos per line end are 0 to 10"))
    out.append(err(next(vid), {**BASE, "end_photos": -1}, "INVALID_INPUT", "Extra photos per line end are 0 to 10"))
    out.append(err(next(vid), {**BASE, "line_spacing": "0.1 m"}, "OUT_OF_DOMAIN", "Line spacing under 0.5 m"))
    out.append(err(next(vid), {**BASE, "line_spacing": "-5 m"}, "INVALID_INPUT", "Line spacing must be positive"))
    out.append(err(next(vid), {**BASE, "area": BASE["area"][:2]}, "INVALID_INPUT", "An area needs 3 corners"))
    return out


def survey_grid():
    out = []
    vid = iter(f"v{i:03d}" for i in range(6, 100))
    for rect, sp in OLD:
        out.append(ok(SG, next(vid), rect, sp, "30 m", METHOD, METHOD_V))
    out.append(ok(SG, next(vid), PSU_RECT, "8400 ft", "2800 ft", PSU, PSU_V, survey=True))
    out.append(ok(SG, next(vid), PSU_RECT, "8400 ft", "2800 ft", PSU + " (390 without the extra images)", PSU_V, extra=0))
    out.append(ok(SG, next(vid), KSU_RECT, "920 m", "460 m", KSU, KSU_V))
    return out


def main():
    outdir = Path(sys.argv[1])
    outdir.mkdir(parents=True, exist_ok=True)
    for tool, vs in ((IC, image_count()), (SG, survey_grid())):
        (outdir / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
