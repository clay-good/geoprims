#!/usr/bin/env python3
"""Golden vectors for aviation.ifr.hold-entry and drone.ops.part107-altitude,
appended at fresh ids after the published ones (which are frozen).

Hold entry is worked from the AIM 5-3-8 figure as a picture, not from the
tool's relative-angle rule: put the fix at the origin, find the bearing FROM
the fix of where the aircraft comes from (heading + 180), and ask which side of
the two sector lines it lies on -- the holding course through the fix, and the
70-degree line drawn through the fix on the holding side (70 degrees off the
outbound course). The 5-degree zone of flexibility is ICAO PANS-OPS Vol I and
TC AIM RAC 10.5.

Part 107 is 14 CFR 107.51(b) as printed on eCFR (read 2026-09-23): 400 ft AGL,
or, within a 400-foot radius of a structure, 400 ft above its uppermost limit.

Run: python3 tools/vectors/gen_hold_part107.py  (appends only ids not present)
"""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FT = 0.3048


def wrap(x):
    return x % 360.0


def signed(x):
    """(-180, 180]"""
    d = wrap(x)
    return d - 360.0 if d > 180.0 else d


def sector(inbound, heading, left):
    """Sector by position: where the aircraft comes from, measured from the
    outbound course toward the holding side."""
    outbound = wrap(inbound + 180.0)
    came_from = wrap(heading + 180.0)
    # Right turns: the holding side is to the right of the inbound course,
    # which, seen from the fix looking down the outbound course, is the
    # counterclockwise side. Left turns mirror it.
    a = signed(outbound - came_from) if not left else signed(came_from - outbound)
    # Direct: the half-plane on the pattern's side of the 70-degree line,
    # from 110 deg past the outbound course on the non-holding side round to
    # the 70-degree line on the holding side.
    if -110.0 < a < 70.0:
        return "direct", a
    if a > 70.0 or a == -180.0 or a == 180.0:
        # beyond the 70-degree line on the holding side, up to the inbound
        # course extended past the fix
        return ("parallel" if a != 180.0 else "boundary"), a
    return "teardrop", a


def hold(inbound, heading, turns):
    left = turns == "left"
    s, a = sector(inbound, heading, left)
    assert s != "boundary"
    # Distance to each boundary line (in the position angle a): -110, 70, 180.
    near = [b for b in (-110.0, 70.0, 180.0) if min(abs(a - b), 360 - abs(a - b)) <= 5.0]
    oc = wrap(inbound + 180.0)
    tear = wrap(oc + 30.0) if left else wrap(oc - 30.0)
    return s, near, oc, tear


def hold_vec(vid, inbound, heading, turns, alt=None, source=None, rel=None, units="deg"):
    s, near, oc, tear = hold(inbound, heading, turns)
    if alt is not None:
        assert near, (inbound, heading, turns)
    else:
        assert not near, (inbound, heading, turns, near)
    exp = {"ok": True, "result.entry": s}
    tol = {}
    if alt is not None:
        exp["result.alternative"] = alt
    for k, v in (("outbound_course", oc), ("teardrop_heading", tear), ("parallel_heading", oc)):
        exp[f"result.{k}.value"] = v
        tol[f"result.{k}.value"] = {"abs": 1e-9}
    if rel is not None:
        exp["result.relative_angle.value"] = rel
        tol["result.relative_angle.value"] = {"abs": 1e-9}
    inp = {"inbound_course": f"{inbound:g} {units}", "heading": f"{heading:g} {units}", "turns": turns}
    return {
        "id": vid,
        "input": inp,
        "expect": exp,
        "source": source
        or "AIM 5-3-8j FIG 5-3-4 sectors worked by position in Python (tools/vectors/gen_hold_part107.py): the side of the holding course and of the 70-degree line the aircraft comes from",
        "sourceVersion": "AIM basic with Change 3 (July 9, 2026)",
        "tolerance": tol,
    }


FLEX = "AIM 5-3-8j FIG 5-3-4 sectors worked by position in Python (tools/vectors/gen_hold_part107.py), with the 5-degree zone of flexibility on the sector boundaries from ICAO Doc 8168 Vol I and TC AIM RAC 10.5"


def hold_vectors():
    v = []
    # Sector interiors, right turns, several inbound courses.
    for ic, h in [(45, 200), (45, 260), (45, 120), (270, 300), (270, 60), (270, 150), (123, 10), (123, 250)]:
        v.append((ic, h, "right", None, None))
    # Left turns.
    for ic, h in [(45, 250), (45, 150), (45, 330), (180, 90), (180, 300), (180, 20)]:
        v.append((ic, h, "left", None, None))
    out = []
    n = 8
    for ic, h, t, alt, src in v:
        out.append(hold_vec(f"v{n:03d}", ic, h, t, alt, src))
        n += 1
    # The published worked example: TC AIM RAC 10.2 example 2. Missed approach
    # on a track of 234 deg to the ZHZ NDB, "make a right turn and hold at the
    # ZHZ beacon on an inbound track of 234" -- the sector 3 (direct) procedure.
    out.append(hold_vec(f"v{n:03d}", 234, 234, "right", None,
                        "TC AIM (TP 14371E, AIM 2026-1) RAC 10.2 example 2: arriving on track 234 deg at ZHZ, make a right turn and hold on an inbound track of 234 deg (the RAC 10.5 sector 3 direct entry)",
                        rel=0.0))
    out[-1]["sourceVersion"] = "TC AIM 2026-1, effective March 19, 2026"
    n += 1
    # Near a boundary: either entry is acceptable.
    for ic, h, t, alt in [
        (360, 107, "right", "teardrop"), (360, 113, "right", "direct"),
        (360, 293, "right", "parallel"), (360, 287, "right", "direct"),
        (360, 176, "right", "parallel"), (360, 184, "right", "teardrop"),
        (360, 253, "left", "teardrop"), (360, 73, "left", "direct"),
    ]:
        vec = hold_vec(f"v{n:03d}", ic, h, t, alt, FLEX, rel=signed(h - ic))
        out.append(vec)
        n += 1
    # Exactly on the reciprocal of the inbound course: the tool's convention is
    # the teardrop for both turn directions, with the parallel also acceptable.
    for t in ("right", "left"):
        out.append({
            "id": f"v{n:03d}",
            "input": {"inbound_course": "090 deg", "heading": "270 deg", "turns": t},
            "expect": {"ok": True, "result.entry": "teardrop", "result.alternative": "parallel",
                       "result.relative_angle.value": 180.0},
            "source": FLEX + "; exactly on the teardrop/parallel line the tool names the teardrop for either turn direction, so a left hold mirrors a right one",
            "sourceVersion": "AIM basic with Change 3 (July 9, 2026)",
            "tolerance": {"result.relative_angle.value": {"abs": 1e-9}},
        })
        n += 1
    # Radians in, degrees out: 3 rad = 171.887 deg, teardrop on a right hold
    # inbound 000, nowhere near a boundary.
    h = math.degrees(3.0)
    s, near, oc, tear = hold(0.0, h, "right")
    assert s == "teardrop" and not near
    out.append({
        "id": f"v{n:03d}",
        "input": {"inbound_course": "0 deg", "heading": "3 rad", "turns": "right"},
        "expect": {"ok": True, "result.entry": "teardrop", "result.relative_angle.value": h,
                   "result.teardrop_heading.value": tear},
        "source": "AIM 5-3-8j FIG 5-3-4 sectors worked by position in Python (tools/vectors/gen_hold_part107.py); 3 rad = 171.887 deg",
        "sourceVersion": "AIM basic with Change 3 (July 9, 2026)",
        "tolerance": {"result.relative_angle.value": {"abs": 1e-9}, "result.teardrop_heading.value": {"abs": 1e-9}},
    })
    n += 1
    # Errors: the heading and the inbound course are required.
    for inp, field in [({"inbound_course": "360 deg"}, "/heading"), ({"heading": "90 deg"}, "/inbound_course")]:
        out.append({
            "id": f"v{n:03d}",
            "input": inp,
            "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": field},
            "source": "Required inputs: an entry needs both the inbound course and the heading to the fix",
            "sourceVersion": "AIM basic with Change 3 (July 9, 2026)",
            "tolerance": {},
        })
        n += 1
    return out


P107_SRC = "14 CFR 107.51(b) as printed on eCFR (current through 2026-09-21), applied in Python (tools/vectors/gen_hold_part107.py): 400 ft AGL, or within a 400-foot radius of a structure up to 400 ft above its immediate uppermost limit"
P107_VER = "eCFR 14 CFR 107.51, last amended 2016-12-30, read 2026-09-23"


def p107(h=None, d=None, ground=None, geoid=None):
    """Returns (agl, msl, hae) in feet."""
    agl = 400.0
    if h is not None and d is not None and d <= 400.0:
        agl = h + 400.0
    msl = None if ground is None else ground + agl
    hae = None if (msl is None or geoid is None) else (msl * FT + geoid) / FT
    return agl, msl, hae


def p107_vectors():
    cases = [
        # (input, h_ft, d_ft, ground_ft, geoid_m)
        ({"structure_height": "0 ft", "structure_distance": "0 ft"}, 0, 0, None, None),
        ({"structure_height": "1000 ft", "structure_distance": "400 ft"}, 1000, 400, None, None),
        ({"structure_height": "1000 ft", "structure_distance": "400.1 ft"}, 1000, 400.1, None, None),
        ({"structure_height": "250 ft", "structure_distance": "0 ft"}, 250, 0, None, None),
        ({"structure_height": "100 m", "structure_distance": "121.92 m"}, 100 / FT, 121.92 / FT, None, None),
        ({"structure_height": "100 m", "structure_distance": "122 m"}, 100 / FT, 122 / FT, None, None),
        ({"structure_height": "300 ft", "structure_distance": "200 ft", "ground_elevation": "1000 ft"}, 300, 200, 1000, None),
        ({"ground_elevation": "-282 ft"}, None, None, -282, None),
        ({"ground_elevation": "5280 ft", "geoid_height": "-17 m"}, None, None, 5280, -17.0),
        ({"structure_height": "500 ft", "structure_distance": "100 ft", "ground_elevation": "1200 ft", "geoid_height": "-30.5 m"}, 500, 100, 1200, -30.5),
        ({"structure_height": "500 ft", "structure_distance": "600 ft", "ground_elevation": "1200 ft", "geoid_height": "25 m"}, 500, 600, 1200, 25.0),
        ({"structure_height": "0.5 mi", "structure_distance": "0.05 mi"}, 0.5 * 5280, 0.05 * 5280, None, None),
    ]
    out = []
    n = 6
    for inp, h, d, g, geo in cases:
        agl, msl, hae = p107(h, d, g, geo)
        exp = {"ok": True, "result.max_agl.value": agl}
        tol = {"result.max_agl.value": {"rel": 1e-9, "abs": 1e-9}}
        if msl is not None:
            exp["result.max_msl.value"] = msl
            tol["result.max_msl.value"] = {"rel": 1e-9, "abs": 1e-9}
        if hae is not None:
            exp["result.max_hae.value"] = hae
            tol["result.max_hae.value"] = {"rel": 1e-9, "abs": 1e-9}
        out.append({"id": f"v{n:03d}", "input": inp, "expect": exp, "source": P107_SRC,
                    "sourceVersion": P107_VER, "tolerance": tol})
        n += 1
    # The regulation's own numbers, with nothing else given: 400 ft AGL, to the
    # foot the rule prints.
    out.append({"id": f"v{n:03d}", "input": {}, "expect": {"ok": True, "result.max_agl.value": 400},
                "source": "14 CFR 107.51(b), eCFR: \"cannot be higher than 400 feet above ground level\" (the rule's printed value, no structure nearby)",
                "sourceVersion": P107_VER, "tolerance": {"result.max_agl.value": {"abs": 0.5}}})
    n += 1
    for inp, field in [
        ({"structure_height": "300 ft"}, "/structure_distance"),
        ({"structure_distance": "100 ft"}, "/structure_distance"),
        ({"structure_height": "-10 ft", "structure_distance": "100 ft"}, "/structure_height"),
        ({"structure_height": "300 ft", "structure_distance": "-5 ft"}, "/structure_distance"),
    ]:
        out.append({"id": f"v{n:03d}", "input": inp,
                    "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": field},
                    "source": "14 CFR 107.51(b) needs both the structure and your distance from it; neither can be negative",
                    "sourceVersion": P107_VER, "tolerance": {}})
        n += 1
    return out


def append(tool, vectors):
    path = ROOT / "core/vectors" / f"{tool}.jsonl"
    lines = path.read_text().splitlines()
    have = {json.loads(l)["id"] for l in lines if l.strip()}
    added = [v for v in vectors if v["id"] not in have]
    with path.open("a") as f:
        for v in added:
            f.write(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(f"{tool}: appended {len(added)}")


if __name__ == "__main__":
    append("aviation.ifr.hold-entry", hold_vectors())
    append("drone.ops.part107-altitude", p107_vectors())
