#!/usr/bin/env python3
"""Golden vectors for the DME tools, worked by hand: ground distance
sqrt(DME^2 - h^2) with h in NM (1 NM = 1852 m, 1 ft = 0.3048 m); time to
station t / sin(change) from where the timing ends and 60 t / change by
the rule; a standard-rate turn radius GS / (60 pi) and the arc lead
asin(r / (R - r)), with 60 r / R by the rule."""
import json
import math
import sys
from pathlib import Path

SRC = "Right-triangle and turn geometry worked in Python (tools/vectors/gen_dme.py)"
SPEC = "add-practitioner-essentials scenarios"
FT_NM = 0.3048 / 1852


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def slant():
    out = []
    cases = [
        (5.0, 6000),
        (10, 3000),
        (2.5, 10000),
        (30, 35000),
        (1.2, 7000),
        (20, 0),
    ]
    for dme, ft in cases:
        h = ft * FT_NM
        g = math.sqrt(dme * dme - h * h)
        out.append(vec(len(out) + 1, {"dme": f"{dme} NM", "height": f"{ft} ft"},
                       {"result.ground_distance.value": g, "result.slant_error.value": dme - g, "result.elevation_angle.value": math.degrees(math.asin(h / dme))}))
    out[0]["source"] = SPEC + " (slant range: 4.902 NM)"
    out.append(vec(len(out) + 1, {"dme": "0.8 NM", "height": "6000 ft"}, {"ok": False, "error.code": "NO_SOLUTION"}, SPEC + " (overhead)"))

    cases = [
        (5.0, 13000),
        (40, 10000),
        (60, 10000),
        (15, 15000),
        (8, 20000),
        (25, 5000),
        (3, 12000),
        (12, 25000),
        (50, 45000),
        (6, 18000),
    ]
    for dme, ft in cases:
        h = ft * FT_NM
        g = math.sqrt(dme * dme - h * h)
        out.append(vec(len(out) + 1, {"dme": f"{dme} NM", "height": f"{ft} ft"},
                       {"result.ground_distance.value": g, "result.slant_error.value": dme - g, "result.elevation_angle.value": math.degrees(math.asin(h / dme))}))
    out[7]["expect"]["result.ground_distance.value"] = 4.52
    out[7]["source"] = "FAA Order 8260.3G paragraph 16-4-1 (5 NM at 13,000 ft gives 4.52 NM horizontally)"
    out[7]["sourceVersion"] = "FAA Order 8260.3G (2024)"
    out[7]["tolerance"]["result.ground_distance.value"] = {"rel": 0.001}

    # The same FAA case in metric units checks input conversion separately.
    dme, height_m = 9.26, 3962.4
    h = height_m / 1852
    g = math.sqrt((dme / 1.852) ** 2 - h * h)
    out.append(vec(len(out) + 1, {"dme": f"{dme} km", "height": f"{height_m} m"},
                   {"result.ground_distance.value": g, "result.slant_error.value": dme / 1.852 - g,
                    "result.elevation_angle.value": math.degrees(math.asin(h / (dme / 1.852)))}))
    out.append(vec(len(out) + 1, {"dme": "1852 m", "height": "1852 m"},
                   {"result.ground_distance.value": 0, "result.slant_error.value": 1, "result.elevation_angle.value": 90}))
    out.append(vec(len(out) + 1, {"dme": "0 NM", "height": "0 ft"},
                   {"result.ground_distance.value": 0, "result.slant_error.value": 0, "result.elevation_angle.value": 90}))
    out.append(vec(len(out) + 1, {"dme": "-1 NM", "height": "0 ft"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/dme"}))
    out.append(vec(len(out) + 1, {"dme": "5 NM", "height": "-1 ft"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/height"}))
    return out


def tts():
    out = []
    for t, d, gs in [(2, 10, 120), (1.5, 5, 90), (3, 15, 150), (4, 20, None), (1, 1, 100), (2.25, 12.5, 135)]:
        exact, rule = t / math.sin(math.radians(d)), 60 * t / d
        inp = {"minutes": f"{t} min", "bearing_change": f"{d} deg"}
        exp = {"result.time.value": exact, "result.time_rule.value": rule}
        if gs:
            inp["groundspeed"] = f"{gs} kt"
            exp.update({"result.distance.value": gs * exact / 60, "result.distance_rule.value": gs * rule / 60})
        out.append(vec(len(out) + 1, inp, exp))
    out.append(vec(len(out) + 1, {"minutes": "2 min", "bearing_change": "95 deg"}, {"ok": False, "error.code": "INVALID_INPUT"}))

    cases = [
        (1.25, 10, 120, {"minutes": "75 s", "bearing_change": "10 deg", "groundspeed": "120 kt"}),
        (0.5, 5, 60, {"minutes": "30 s", "bearing_change": "5 deg", "groundspeed": "60 kt"}),
        (1.5, 15, 180, {"minutes": "90 s", "bearing_change": "15 deg", "groundspeed": "180 kt"}),
        (2.5, 20, 110, {"minutes": "2.5 min", "bearing_change": "20 deg", "groundspeed": "110 kt"}),
        (0.75, 8, None, {"minutes": "45 s", "bearing_change": "8 deg"}),
        (1, 30, 200, {"minutes": "1 min", "bearing_change": "30 deg", "groundspeed": "200 kt"}),
        (3, 45, 75, {"minutes": "3 min", "bearing_change": "45 deg", "groundspeed": "75 kt"}),
        (0.5, 89, 100, {"minutes": "0.5 min", "bearing_change": "89 deg", "groundspeed": "100 kt"}),
        (2, 10, 120, {"minutes": "120 s", "bearing_change": "0.17453292519943295 rad", "groundspeed": "61.73333333333333 m/s"}),
        (1 / 60, 0.1, 1, {"minutes": "1 s", "bearing_change": "0.1 deg", "groundspeed": "1 kt"}),
    ]
    for t, d, gs, inp in cases:
        exact, rule = t / math.sin(math.radians(d)), 60 * t / d
        exp = {"result.time.value": exact, "result.time_rule.value": rule}
        if gs is not None:
            exp.update({"result.distance.value": gs * exact / 60, "result.distance_rule.value": gs * rule / 60})
        out.append(vec(len(out) + 1, inp, exp))
    out[7]["source"] = "FAA Pilot's Handbook of Aeronautical Knowledge FAA-H-8083-25C, chapter 16, Figure 16-33 and following text (75 seconds over 10° gives 7.5 minutes; at 120 kt, 15 NM)"
    out[7]["sourceVersion"] = "FAA-H-8083-25C (2023)"

    out.append(vec(len(out) + 1, {"minutes": "0 min", "bearing_change": "10 deg"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/minutes"}))
    out.append(vec(len(out) + 1, {"minutes": "1 min", "bearing_change": "0 deg"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/bearing_change"}))
    out.append(vec(len(out) + 1, {"minutes": "1 min", "bearing_change": "10 deg", "groundspeed": "0 kt"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/groundspeed"}))
    return out


def arc():
    out = []
    for big_r, gs, r in [(10, 120, None), (15, 150, None), (7, 90, None), (12, 200, None), (10, 120, 1.0), (20, 250, None)]:
        rr = r if r is not None else gs / (60 * math.pi)
        inp = {"arc": f"{big_r} NM", "groundspeed": f"{gs} kt"}
        if r is not None:
            inp["turn_radius"] = f"{r} NM"
        out.append(vec(len(out) + 1, inp, {"result.turn_radius.value": rr, "result.onto_arc_lead.value": rr,
                                           "result.lead_radials.value": math.degrees(math.asin(rr / (big_r - rr))), "result.lead_radials_rule.value": 60 * rr / big_r}))
    out.append(vec(len(out) + 1, {"arc": "1 NM", "groundspeed": "200 kt"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    out.append(vec(len(out) + 1, {"arc": "12 NM", "groundspeed": "180 kt", "turn_radius": "0.9 NM"},
                   {"result.turn_radius.value": 0.9, "result.onto_arc_lead.value": 0.9,
                    "result.lead_radials.value": math.degrees(math.asin(0.9 / 11.1)), "result.lead_radials_rule.value": 4.5},
                   "CNATRA P-765 Change 4, chapter 6 section 607 (0.9 NM turn radius and 4.5 lead radials)", 1e-9))
    for big_r, gs, r, units in [
        (8, 100, None, False), (9, 135, None, False), (12, 180, None, False),
        (18, 210, None, False), (25, 300, None, False), (40, 240, None, False),
        (14, 160, 0.75, False), (30, 220, 1.4, False), (10, 120, None, True),
    ]:
        rr = r if r is not None else gs / (60 * math.pi)
        inp = {"arc": f"{big_r * 1.852} km" if units else f"{big_r} NM",
               "groundspeed": f"{gs * 0.5144444444444445} m/s" if units else f"{gs} kt"}
        if r is not None:
            inp["turn_radius"] = f"{r} NM"
        out.append(vec(len(out) + 1, inp, {"result.turn_radius.value": rr, "result.onto_arc_lead.value": rr,
                                           "result.lead_radials.value": math.degrees(math.asin(rr / (big_r - rr))),
                                           "result.lead_radials_rule.value": 60 * rr / big_r}))
    out.append(vec(len(out) + 1, {"arc": "0 NM", "groundspeed": "120 kt"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/arc"}))
    out.append(vec(len(out) + 1, {"arc": "10 NM", "groundspeed": "0 kt"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/groundspeed"}))
    out.append(vec(len(out) + 1, {"arc": "10 NM", "groundspeed": "120 kt", "turn_radius": "0 NM"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/turn_radius"}))
    return out


def intercept():
    """The heading is whichever of course ± the intercept angle actually reaches
    the wanted radial: flown 10 NM out from the station on the current radial,
    its track must cross the radial's ray (not its extension past the station)."""
    out = []
    for now, want, inbound, given in [(30, 360, True, None), (30, 360, False, None), (350, 20, True, None), (100, 90, False, None),
                                      (200, 250, True, 45), (275, 270, True, None), (5, 355, False, 30), (180, 180, True, None)]:
        course = (want + 180) % 360 if inbound else want % 360
        off = (now - want + 180) % 360 - 180
        angle = given if given is not None else min(90, max(20, 2 * abs(off)))
        px, py = 10 * math.sin(math.radians(now)), 10 * math.cos(math.radians(now))
        ux, uy = math.sin(math.radians(want)), math.cos(math.radians(want))
        heading = course if off == 0 else None
        for h in ((course + angle) % 360, (course - angle) % 360):
            dx, dy = math.sin(math.radians(h)), math.cos(math.radians(h))
            den = dx * uy - dy * ux
            if abs(den) < 1e-12:
                continue
            t = (ux * py - uy * px) / den  # along the track to the radial's line
            r = (dx * py - dy * px) / den  # along the radial from the station
            if off != 0 and t > 0 and r > 0:
                heading = h
        inp = {"current_radial": f"{now} deg", "desired_radial": f"{want} deg", "direction": "inbound" if inbound else "outbound"}
        if given is not None:
            inp["intercept_angle"] = f"{given} deg"
        if heading is None:
            # Neither heading meets the radial before the station.
            out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "NO_SOLUTION"}))
            continue
        out.append(vec(len(out) + 1, inp, {"result.heading.value": float(heading), "result.course.value": float(course),
                                           "result.intercept_angle.value": float(angle), "result.angle_off.value": float(abs(off))},
                       "Plane geometry: the heading of course ± the angle whose track crosses the radial (tools/vectors/gen_dme.py)"))
    out.append(vec(len(out) + 1, {"current_radial": "200 deg", "desired_radial": "360 deg"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    # FAA-H-8083-15B chapter 9, Figure 9-16: on the 160 radial,
    # intercept the 205 radial inbound with a 90° angle on heading 295°.
    out.append(vec(len(out) + 1,
                   {"current_radial": "160 deg", "desired_radial": "205 deg", "direction": "inbound"},
                   {"result.heading.value": 295.0, "result.course.value": 25.0,
                    "result.intercept_angle.value": 90.0, "result.angle_off.value": 45.0},
                   "FAA-H-8083-15B chapter 9, Figure 9-16 (heading 295° to intercept the 205 radial inbound)"))
    more = [
        (10, 350, True, 45), (45, 60, False, None), (359, 1, True, None),
        (1, 359, True, None), (90, 120, False, 90), (270, 225, True, 60),
        (0, 0, False, None),
    ]
    for now, want, inbound, given in more:
        course = (want + 180) % 360 if inbound else want % 360
        off = (now - want + 180) % 360 - 180
        angle = given if given is not None else min(90, max(20, 2 * abs(off)))
        heading = course if off == 0 else (course + (1 if off > 0 else -1) * angle if inbound
                                          else course - (1 if off > 0 else -1) * angle) % 360
        inp = {"current_radial": f"{now} deg", "desired_radial": f"{want} deg",
               "direction": "inbound" if inbound else "outbound"}
        if given is not None:
            inp["intercept_angle"] = f"{given} deg"
        out.append(vec(len(out) + 1, inp, {"result.heading.value": float(heading),
                                           "result.course.value": float(course),
                                           "result.intercept_angle.value": float(angle),
                                           "result.angle_off.value": float(abs(off))}))
    out.append(vec(len(out) + 1, {"current_radial": "100 deg", "desired_radial": "90 deg",
                                  "direction": "inbound", "intercept_angle": "10 deg"},
                   {"ok": False, "error.code": "NO_SOLUTION", "error.field": "/intercept_angle"}))
    out.append(vec(len(out) + 1, {"current_radial": "0 deg", "desired_radial": "90 deg", "direction": "inbound"},
                   {"ok": False, "error.code": "NO_SOLUTION", "error.field": "/intercept_angle"}))
    out.append(vec(len(out) + 1, {"current_radial": "0 deg", "desired_radial": "0 deg",
                                  "direction": "outbound", "intercept_angle": "5 deg"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/intercept_angle"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.ifr.dme-slant-range", slant()), ("aviation.ifr.time-to-station", tts()), ("aviation.ifr.dme-arc-lead", arc()), ("aviation.ifr.radial-intercept", intercept())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
