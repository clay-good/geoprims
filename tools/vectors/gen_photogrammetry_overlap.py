#!/usr/bin/env python3
"""Appends the stable-promotion vectors for drone.photogrammetry.trigger and
drone.photogrammetry.terrain-overlap.

Expected values come from the published relations, evaluated here without the
core: footprint = sensor size x height / focal length, trigger distance =
footprint along x (1 - front overlap), line spacing = footprint across x
(1 - side overlap), interval = distance / groundspeed (Wolf, Dewitt, and
Wilkinson, 4th edition, ch. 18; Penn State GEOG 892), and the overlap left over
ground t above the takeoff datum with spacing fixed at height h,
o_t = 1 - (1 - o) h / (h - t) (Pryor 1959, HRB Bulletin 228, pp. 36-37).

Published vectors are frozen: this script only appends ids that are not in
the file yet, so re-running it changes nothing.

    python3 tools/vectors/gen_photogrammetry_overlap.py
"""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FT = 0.3048
KT = 1852.0 / 3600.0
MPH = 0.44704
KMH = 1.0 / 3.6
FULL_FRAME_DIAGONAL = math.hypot(36.0, 24.0)
TIGHT = {"rel": 1e-9, "abs": 1e-9}

TRIGGER_SRC = "Flight-planning relations (Wolf, Dewitt & Wilkinson 2014, ch. 18) evaluated in Python (tools/vectors/gen_photogrammetry_overlap.py)"
TERRAIN_SRC = "Overlap at the highest relief, o_t = 1 - (1 - o) h / (h - t) (Pryor 1959, HRB Bulletin 228, pp. 36-37), evaluated in Python (tools/vectors/gen_photogrammetry_overlap.py)"


def trigger(sw, sh, f, h, v, front, side, portrait=False):
    """sw, sh, f in mm; h in m; v in m/s; overlaps in percent."""
    across, along = sw * h / f, sh * h / f
    if portrait:
        across, along = along, across
    d = along * (1 - front / 100)
    return {"trigger_distance": d, "trigger_interval": d / v, "line_spacing": across * (1 - side / 100),
            "footprint_along": along, "footprint_across": across}


def terrain(h, t, o):
    return 100 * (1 - (1 - o / 100) * h / (h - t))


def min_height(t, planned, target):
    return max(t * (1 - target / 100) / (p / 100 - target / 100) for p in planned)


def vec(expect, tol=None, src=TRIGGER_SRC, ver="4th edition (2014)", **inp):
    e = {f"result.{k}.value" if not k.startswith(("meta.", "result.", "ok", "error.")) else k: x for k, x in expect.items()}
    t = {k: (tol if tol is not None else TIGHT) for k in e if k.startswith("result.")}
    return {"input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


ONE_INCH = dict(sensor_width="13.2 mm", sensor_height="8.8 mm", focal_length="8.8 mm", image_width=5472)
M3E = dict(sensor_width="17.3 mm", sensor_height="13.0 mm", focal_length="12.29 mm", image_width=5280)


def pick(r, *keys):
    return {k: r[k] for k in keys}


def trigger_vectors():
    out = []
    # Penn State GEOG 892, "Designing a Flight Route": 12,000 x 7,000 px of
    # 10 um (120 x 70 mm), f = 100 mm, 1 ft GSD at 10,000 ft, 60/30, 150 kn.
    # Published: B = 2,800 ft, SP = 8,400 ft, t = 11.067 s (150 kn x 1.15 mph).
    out.append({
        "input": {"height": "10000 ft", "sensor_width": "120 mm", "sensor_height": "70 mm", "focal_length": "100 mm",
                  "image_width": 12000, "groundspeed": "150 kn", "front_overlap": 60, "side_overlap": 30},
        "expect": {"result.trigger_distance.value": 2800 * FT, "result.line_spacing.value": 8400 * FT,
                   "result.footprint_across.value": 12000 * FT, "result.footprint_along.value": 7000 * FT,
                   "result.trigger_interval.value": 11.067, "ok": True},
        "source": "Penn State GEOG 892 (Q. Abdullah), Designing a Flight Route: air base 2,800 ft, line spacing 8,400 ft, 11.067 s between exposures",
        "sourceVersion": "Penn State College of Earth and Mineral Sciences, retrieved 2026-09-23",
        "tolerance": {"result.trigger_distance.value": {"abs": 0.01}, "result.line_spacing.value": {"abs": 0.01},
                      "result.footprint_across.value": {"abs": 0.01}, "result.footprint_along.value": {"abs": 0.01},
                      "result.trigger_interval.value": {"abs": 0.01}},
    })
    # University of Washington CEE 424, Flight Planning sheet 1, problems 6-7:
    # f = 305 mm, 230 mm format, 1:15,000 (H = 4,575 m), 60/25, 260 km/h.
    # Published: B = 1,380 m, S = 2,587.5 m, 19.12 s between exposures.
    out.append({
        "input": {"height": "4575 m", "sensor_width": "230 mm", "sensor_height": "230 mm", "focal_length": "305 mm",
                  "image_width": 11500, "groundspeed": "260 km/h", "front_overlap": 60, "side_overlap": 25},
        "expect": {"result.trigger_distance.value": 1380.0, "result.line_spacing.value": 2587.5,
                   "result.trigger_interval.value": 19.12, "ok": True},
        "source": "University of Washington CEE 424 (K. M. Ahmed), Flight Planning sheet 1, problems 6 and 7",
        "sourceVersion": "courses.washington.edu/cee424, retrieved 2026-09-23",
        "tolerance": {"result.trigger_distance.value": {"abs": 0.5}, "result.line_spacing.value": {"abs": 0.5},
                      "result.trigger_interval.value": {"abs": 0.02}},
    })
    r = trigger(13.2, 8.8, 8.8, 100, 10, 75, 65, portrait=True)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "footprint_along", "footprint_across") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=75, side_overlap=65, orientation="portrait", **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 75, 60)  # no overlap given: the general preset
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "trigger_interval") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 85, 70)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "trigger_interval") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", preset="forest", **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 80, 60)  # a given overlap overrides the preset's
    out.append(vec(pick(r, "trigger_distance", "line_spacing") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", preset="general", front_overlap=80, **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 75, 65)
    out.append(vec({"trigger_interval": r["trigger_interval"], "max_groundspeed": r["trigger_distance"] / 3.0,
                    "meta.warnings.*.code": "TRIGGER_TOO_FAST", "ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=75, side_overlap=65, min_interval="3 s", **ONE_INCH))
    out.append(vec({"trigger_interval": r["trigger_interval"], "max_groundspeed": r["trigger_distance"] / 2.0, "ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=75, side_overlap=65, min_interval="2 s", **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 0, 0)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "footprint_along", "footprint_across") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=0, side_overlap=0, **ONE_INCH))
    r = trigger(13.2, 8.8, 8.8, 100, 10, 99, 99)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "trigger_interval") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=99, side_overlap=99, **ONE_INCH))
    r = trigger(17.3, 13.0, 12.29, 400 * FT, 15 * MPH, 70, 70)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "trigger_interval") | {"ok": True},
                   height="400 ft", groundspeed="15 mph", front_overlap=70, side_overlap=70, **M3E))
    fe = 24.0 / (FULL_FRAME_DIAGONAL / math.hypot(13.2, 8.8))
    r = trigger(13.2, 8.8, fe, 100, 10, 75, 65)
    out.append(vec(pick(r, "trigger_distance", "line_spacing") | {"ok": True},
                   height="100 m", groundspeed="10 m/s", front_overlap=75, side_overlap=65,
                   focal_length_type="equivalent-35mm", **(ONE_INCH | {"focal_length": "24 mm"})))
    r = trigger(6.17, 4.55, 4.5, 50, 12 * KT, 80, 70)
    out.append(vec(pick(r, "trigger_distance", "line_spacing", "trigger_interval") | {"ok": True},
                   sensor_width="6.17 mm", sensor_height="4.55 mm", focal_length="4.5 mm", image_width=4000,
                   height="50 m", groundspeed="12 kn", front_overlap=80, side_overlap=70))
    out.append(vec({"ok": False, "error.code": "INVALID_INPUT"}, height="0 m", groundspeed="10 m/s", **ONE_INCH))
    out.append(vec({"ok": False, "error.code": "INVALID_INPUT"}, height="100 m", groundspeed="0 m/s", **ONE_INCH))
    out.append(vec({"ok": False, "error.code": "INVALID_INPUT"}, height="100 m", groundspeed="10 m/s", min_interval="0 s", **ONE_INCH))
    out.append(vec({"ok": False, "error.code": "INVALID_INPUT"}, height="100 m", groundspeed="10 m/s",
                   sensor_width="13.2 mm", focal_length="8.8 mm", image_width=5472))
    return out


def terrain_vectors(old):
    out = []
    # Replacements for the vectors that pinned OVERLAP_BELOW_TARGET at index 1.
    for v in old:
        if not any(k.startswith("meta.warnings.1.") for k in v["expect"]):
            continue
        i = v["input"]
        h = float(i["height"].split()[0])
        t = float(i["highest_terrain"].split()[0])
        e = {"result.effective_height.value": h - t, "result.front_overlap_worst": terrain(h, t, i["front_overlap"])}
        if "side_overlap" in i:
            e["result.side_overlap_worst"] = terrain(h, t, i["side_overlap"])
        planned = [i["front_overlap"]] + ([i["side_overlap"]] if "side_overlap" in i else [])
        if "target_overlap" in i and t > 0 and all(i["target_overlap"] < p for p in planned):
            e["result.min_height.value"] = min_height(t, planned, i["target_overlap"])
        e |= {"meta.warnings.*.code": "OVERLAP_BELOW_TARGET", "ok": True}
        out.append((v["id"], {"input": i, "expect": e, "source": TERRAIN_SRC, "sourceVersion": "Bulletin 228 (1959)",
                              "tolerance": {k: TIGHT for k in e if k.startswith("result.")}}))
    new = []
    # Pryor p. 31: 55/65 endlap limits at 20,000 ft accommodate 4,444 ft of relief.
    new.append({"input": {"height": "20000 ft", "highest_terrain": "4444 ft", "front_overlap": 65, "target_overlap": 55},
                "expect": {"result.front_overlap_worst": 55.0, "result.min_height.value": 20000 * FT, "ok": True},
                "source": "Pryor 1959, Relationship of Topographic Relief, Flight Height, and Minimum and Maximum Overlap, HRB Bulletin 228, p. 31",
                "sourceVersion": "Bulletin 228 (1959)",
                "tolerance": {"result.front_overlap_worst": {"abs": 0.01}, "result.min_height.value": {"abs": 1.0}}})
    # Pryor p. 32: at 1,300 ft with 65% maximum endlap, 2/9 of 1,300 = 289 ft of relief.
    new.append({"input": {"height": "1300 ft", "highest_terrain": "289 ft", "front_overlap": 65, "target_overlap": 55},
                "expect": {"result.front_overlap_worst": 55.0, "result.min_height.value": 1300 * FT, "ok": True},
                "source": "Pryor 1959, Relationship of Topographic Relief, Flight Height, and Minimum and Maximum Overlap, HRB Bulletin 228, p. 32",
                "sourceVersion": "Bulletin 228 (1959)",
                "tolerance": {"result.front_overlap_worst": {"abs": 0.01}, "result.min_height.value": {"abs": 0.5}}})
    h, t = 400 * FT, 100 * FT
    new.append({"input": {"height": "400 ft", "highest_terrain": "100 ft", "front_overlap": 80, "side_overlap": 70,
                          "target_overlap": 65},
                "expect": {"result.effective_height.value": h - t, "result.front_overlap_worst": terrain(h, t, 80),
                           "result.side_overlap_worst": terrain(h, t, 70),
                           "result.min_height.value": min_height(t, [80, 70], 65), "ok": True},
                "source": TERRAIN_SRC, "sourceVersion": "Bulletin 228 (1959)", "tolerance": {}})
    h, t = 100.0, 20.0  # the accepted overlap is not below the plan: no height to replan to
    new.append({"input": {"height": "100 m", "highest_terrain": "20 m", "front_overlap": 75, "target_overlap": 80},
                "expect": {"result.front_overlap_worst": terrain(h, t, 75), "meta.warnings.*.code": "OVERLAP_BELOW_TARGET",
                           "ok": True},
                "source": TERRAIN_SRC, "sourceVersion": "Bulletin 228 (1959)", "tolerance": {}})
    h, t = 100.0, -30.0  # a valley: overlap and GSD both grow
    pitch = 13.2e-3 / 5472
    new.append({"input": {"height": "100 m", "highest_terrain": "-30 m", "front_overlap": 75, "side_overlap": 65,
                          "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472},
                "expect": {"result.front_overlap_worst": terrain(h, t, 75), "result.side_overlap_worst": terrain(h, t, 65),
                           "result.gsd_worst.value": pitch * (h - t) / 8.8e-3 * 100,
                           "result.gsd_takeoff.value": pitch * h / 8.8e-3 * 100, "ok": True},
                "source": TERRAIN_SRC, "sourceVersion": "Bulletin 228 (1959)", "tolerance": {}})
    new.append({"input": {"height": "0 m", "highest_terrain": "0 m", "front_overlap": 75},
                "expect": {"ok": False, "error.code": "INVALID_INPUT"}, "source": TERRAIN_SRC,
                "sourceVersion": "Bulletin 228 (1959)", "tolerance": {}})
    new.append({"input": {"height": "100 m", "highest_terrain": "120 m", "front_overlap": 75},
                "expect": {"ok": False, "error.code": "INVALID_INPUT"}, "source": TERRAIN_SRC,
                "sourceVersion": "Bulletin 228 (1959)", "tolerance": {}})
    for v in new:
        if not v["tolerance"]:
            v["tolerance"] = {k: TIGHT for k in v["expect"] if k.startswith("result.")}
    return out, new


def load(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def dump(vs):
    return "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs)


def next_id(vs):
    return max(int(v["id"][1:]) for v in vs) + 1


def main():
    tp = ROOT / "core/vectors/drone.photogrammetry.trigger.jsonl"
    tv = load(tp)
    if len(tv) == 5:
        n = next_id(tv)
        for k, v in enumerate(trigger_vectors()):
            tv.append({"id": f"v{n + k:03d}"} | v)
        tp.write_text(dump(tv))
    op = ROOT / "core/vectors/drone.photogrammetry.terrain-overlap.jsonl"
    ov = load(op)
    if len(ov) == 14:
        replaced, new = terrain_vectors(ov)
        n = next_id(ov)
        by = {v["id"]: v for v in ov}
        for old_id, v in replaced:
            nid = f"v{n:03d}"
            n += 1
            by[old_id]["supersededBy"] = nid
            by[old_id]["reason"] = "Promotion to stable dropped EXPERIMENTAL_TOOL, which moved OVERLAP_BELOW_TARGET from index 1 to 0; the replacement names the warning by code."
            ov.append({"id": nid} | v)
        for v in new:
            ov.append({"id": f"v{n:03d}"} | v)
            n += 1
        op.write_text(dump(ov))


if __name__ == "__main__":
    main()
