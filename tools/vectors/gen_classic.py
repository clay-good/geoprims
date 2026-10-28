#!/usr/bin/env python3
"""Golden vectors for the classic photogrammetry tools (add-photogrammetry-
classics), worked in Python from USACE EM 1110-1-1000 (1993) chapter 10:
scale S = f / (H - h) (10-1), D = d (H - h) / f (10-3), relief displacement
h = d (H - h_base) / r (10-7), parallax h_A = h_C + dp (H - h_C) / p_A with
p_A = p_C + dp (10-15), h = H - f B / p and X = x B / p (10-14), and the air
base B = G (1 - E / 100) with G = frame length x H / f (10-9, 10-11).

    gen_classic.py            # print every tool's vectors as JSON lines
    gen_classic.py TOOL_ID    # print one tool's

Published examples come first in each list, marked with their source. Writes
nothing: new vectors are appended by hand so published files stay frozen.
"""
import json
import sys

FT = 0.3048
IN = 0.0254
SRC = "Worked in Python from EM 1110-1-1000 (1993) ch. 10 (tools/vectors/gen_classic.py)"
VER = "31 March 1993"
REL = 1e-9


def vec(i, inp, exp, source=SRC, version=VER, tol=None):
    tol = tol or {k: {"rel": REL, "abs": 1e-9} for k, v in exp.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": {**exp, "ok": True}, "source": source, "sourceVersion": version, "tolerance": tol}


def bad(i, inp, field):
    return {"id": f"v{i:03d}", "input": inp, "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": field}, "source": SRC, "sourceVersion": VER, "tolerance": {}}


def ratio(n):
    return f"1:{round(n):,}"


def photo_scale():
    out = []
    # Penn State GEOG 892 lesson 4: 6 in lens, 6,000 ft AMT -> 1:12,000.
    above = 6000 * FT
    f = 6 * IN
    out.append(({"focal_length": "6 in", "height": "6000 ft"},
                {"result.scale": "1:12,000", "result.scale_number": above / f, "result.height_above_ground.value": above, "result.flying_height.value": above},
                "Penn State GEOG 892, Lesson 4, Scale from Film Camera (1:12,000)", "online course text"))
    # Penn State lesson 4: 50 mm, 10 um, GSD 0.30 m -> 1,500 m above ground.
    out.append(({"focal_length": "50 mm", "gsd": "30 cm", "pixel_pitch": "10 um"},
                {"result.height_above_ground.value": 0.30 * 0.050 / 10e-6, "result.gsd.value": 30.0},
                "Penn State GEOG 892, Lesson 4, Flight Altitude Computations (1,500 m)", "online course text"))
    cases = [
        (152.4, 1829.0, 180.0, None), (88.0, 1000.0, 0.0, 25.4), (8.8, 120.0, 15.5, 10.0), (35.0, 300.0, 50.0, None),
        (24.0, 90.0, 0.0, 3.2), (210.0, 4500.0, 1250.0, 100.0), (153.0, 2600.0, 600.0, 12.7), (12.3, 150.0, 30.0, None),
        (100.0, 3048.0, 0.0, 70.0), (300.0, 9000.0, 2000.0, 1.0),
    ]
    for f_mm, H, h, d in cases:
        a = H - h
        inp = {"focal_length": f"{f_mm} mm", "height": f"{H} m", "elevation": f"{h} m"}
        exp = {"result.scale": ratio(a / (f_mm / 1000)), "result.scale_number": a / (f_mm / 1000), "result.height_above_ground.value": a, "result.flying_height.value": H}
        if d is not None:
            inp["photo_distance"] = f"{d} mm"
            exp["result.ground_distance.value"] = d / 1000 * a / (f_mm / 1000)
        out.append((inp, exp, None, None))
    for f_mm, n, h in [(152.4, 12000, 0.0), (88.0, 25000, 300.0), (8.8, 5000, 40.0), (50.0, 1000, 0.0), (210.0, 40000, 1000.0)]:
        a = n * f_mm / 1000
        out.append(({"focal_length": f"{f_mm} mm", "scale": n, "elevation": f"{h} m"},
                    {"result.scale": ratio(n), "result.height_above_ground.value": a, "result.flying_height.value": a + h}, None, None))
    for f_mm, H, pitch in [(8.8, 100.0, 2.41), (35.0, 120.0, 4.4), (24.0, 80.0, 3.3)]:
        a = H
        out.append(({"focal_length": f"{f_mm} mm", "height": f"{H} m", "pixel_pitch": f"{pitch} um"},
                    {"result.gsd.value": pitch * 1e-6 * a / (f_mm / 1000) * 100}, None, None))
    vs = [vec(i + 1, inp, exp, src or SRC, ver or VER) for i, (inp, exp, src, ver) in enumerate(out)]
    n = len(vs)
    vs.append(bad(n + 1, {"focal_length": "50 mm"}, "/height"))
    vs.append(bad(n + 2, {"focal_length": "50 mm", "height": "100 m", "scale": 2000}, "/height"))
    vs.append(bad(n + 3, {"focal_length": "50 mm", "gsd": "3 cm"}, "/pixel_pitch"))
    vs.append(bad(n + 4, {"focal_length": "50 mm", "height": "100 m", "elevation": "150 m"}, "/height"))
    return vs


def relief():
    out = []
    # EM 1110-1-1000 eq. 10-7, h_t = d(H - h_base) / r_top, read at the open
    # Arizona copy: 54.1 * (535 - 259) / 121.7 = 122.69 m. Wolf, Dewitt &
    # Wilkinson's example 6-7 carries the same case and prints about 122.7 m,
    # but that book is not open, so the equation is the authority here and the
    # book's rounded figure is only the agreement check.
    out.append(({"displacement": "54.1 mm", "radial_distance": "121.7 mm", "flying_height": "535 m", "base_elevation": "259 m"},
                {"result.object_height.value": 54.1 * (535 - 259) / 121.7},
                "EM 1110-1-1000 (31 Mar 93) eq. 10-7, h_t = d(H - h_base) / r_top: "
                "54.1 * (535 - 259) / 121.7 = 122.69 m, which agrees with the about 122.7 m "
                "of Wolf, Dewitt & Wilkinson example 6-7", "31 March 1993",
                {"result.object_height.value": {"abs": 0.05}}))
    cases = [(54.1, 121.7, 535.0, 259.0), (3.2, 88.0, 1200.0, 0.0), (12.5, 140.0, 2500.0, 400.0), (0.8, 45.0, 600.0, 20.0),
             (20.0, 110.0, 900.0, 150.0), (6.4, 95.5, 1500.0, 300.0), (1.75, 60.2, 450.0, 10.0)]
    for d, r, H, b in cases:
        a = H - b
        h = d * a / r
        base = {"flying_height": f"{H} m", "base_elevation": f"{b} m"}
        out.append(({"displacement": f"{d} mm", "radial_distance": f"{r} mm", **base}, {"result.object_height.value": h, "result.height_above_base.value": a}, None, None, None))
        out.append(({"radial_distance": f"{r} mm", "object_height": f"{h} m", **base}, {"result.displacement.value": r * h / a}, None, None, None))
        out.append(({"displacement": f"{d} mm", "object_height": f"{h} m", **base}, {"result.radial_distance.value": d * a / h}, None, None, None))
    for d, r, h, b in [(54.1, 121.7, 122.69, 259.0), (3.0, 90.0, 40.0, 100.0), (10.0, 120.0, 75.0, 0.0)]:
        out.append(({"displacement": f"{d} mm", "radial_distance": f"{r} mm", "object_height": f"{h} m", "base_elevation": f"{b} m"},
                    {"result.flying_height.value": r * h / d + b, "result.height_above_base.value": r * h / d}, None, None, None))
    vs = [vec(i + 1, inp, exp, src or SRC, ver or VER, tol) for i, (inp, exp, src, ver, tol) in enumerate(out)]
    n = len(vs)
    vs.append(bad(n + 1, {"displacement": "5 mm", "radial_distance": "80 mm"}, "/displacement"))
    vs.append(bad(n + 2, {"displacement": "90 mm", "radial_distance": "80 mm", "flying_height": "500 m"}, "/displacement"))
    vs.append(bad(n + 3, {"displacement": "5 mm", "radial_distance": "80 mm", "flying_height": "100 m", "base_elevation": "200 m"}, "/flying_height"))
    return vs


def parallax():
    out = []
    # EM 1110-1-1000 eq. 10-15, h_A = h_C + dp(H - h_C) / P_a with
    # P_a = P_c + dp: 1.3 * 915 / 89.5 = 13.29 m. Wolf, Dewitt & Wilkinson's
    # example 8-3 prints about 13.3 m for the same case; again the equation is
    # the authority and the book is the check.
    out.append(({"flying_height": "915 m", "control_parallax": "88.2 mm", "parallax_difference": "1.3 mm", "control_elevation": "0 m"},
                {"result.elevation.value": 1.3 * 915 / 89.5},
                "EM 1110-1-1000 (31 Mar 93) eq. 10-15, h_A = h_C + dp(H - h_C) / P_a with "
                "P_a = P_c + dp: 1.3 * 915 / 89.5 = 13.29 m, which agrees with the about 13.3 m "
                "of Wolf, Dewitt & Wilkinson example 8-3", "31 March 1993",
                {"result.elevation.value": {"abs": 0.05}}))
    # The same equation in feet and inches: 0.6 * 4600 / 5.0 = 552 ft, the
    # Washington Monument case that is worked in many photogrammetry courses.
    out.append(({"flying_height": "4600 ft", "control_parallax": "4.4 in", "parallax_difference": "0.6 in"},
                {"result.elevation.value": 0.6 * 4600 / 5.0 * FT},
                "EM 1110-1-1000 (31 Mar 93) eq. 10-15 in feet and inches: 0.6 * 4600 / 5.0 = 552 ft, "
                "the Washington Monument case", "31 March 1993",
                {"result.elevation.value": {"abs": 0.01}}))
    for H, hc, pc, dp in [(915.0, 0.0, 88.2, 1.3), (1500.0, 210.0, 92.4, 2.15), (2400.0, 650.0, 101.7, -1.4), (760.0, 35.0, 70.0, 0.0),
                          (3000.0, 1200.0, 84.3, 3.9), (1200.0, 100.0, 95.0, -0.6), (600.0, 0.0, 66.6, 4.4),
                          (1830.0, 300.0, 90.0, 0.75), (2000.0, 0.0, 80.5, 2.6), (1100.0, 450.0, 72.2, -0.35), (4000.0, 1500.0, 110.4, 5.2)]:
        pa = pc + dp
        h = hc + dp * (H - hc) / pa
        out.append(({"flying_height": f"{H} m", "control_parallax": f"{pc} mm", "parallax_difference": f"{dp} mm", "control_elevation": f"{hc} m"},
                    {"result.elevation.value": h, "result.height_above_control.value": h - hc, "result.point_parallax.value": pa}, None, None, None))
    for H, p, f, B, x, y in [(1830.0, 90.4, 152.4, 600.0, 30.5, -12.2), (1500.0, 75.0, 88.0, 900.0, 10.0, 20.0), (2500.0, 102.3, 153.0, 1000.0, -40.1, 55.0),
                             (900.0, 60.0, 50.0, 700.0, 0.0, 0.0), (3000.0, 110.0, 210.0, 1200.0, 70.0, -5.5)]:
        ratio_ = B / (p / 1000)
        out.append(({"flying_height": f"{H} m", "parallax": f"{p} mm", "focal_length": f"{f} mm", "air_base": f"{B} m", "x": f"{x} mm", "y": f"{y} mm"},
                    {"result.elevation.value": H - f / 1000 * ratio_, "result.ground_x.value": x / 1000 * ratio_, "result.ground_y.value": y / 1000 * ratio_}, None, None, None))
    vs = [vec(i + 1, inp, exp, src or SRC, ver or VER, tol) for i, (inp, exp, src, ver, tol) in enumerate(out)]
    n = len(vs)
    vs.append(bad(n + 1, {"flying_height": "900 m"}, "/parallax_difference"))
    vs.append(bad(n + 2, {"flying_height": "900 m", "control_parallax": "80 mm", "parallax_difference": "1 mm", "control_elevation": "1000 m"}, "/flying_height"))
    vs.append(bad(n + 3, {"flying_height": "900 m", "control_parallax": "1 mm", "parallax_difference": "-2 mm"}, "/parallax_difference"))
    return vs


def stereo():
    out = []
    for H, f, d, E, sp in [(10000 * FT, 100.0, 70.0, 60, 0.005), (10000 * FT, 100.0, 70.0, 80, 0.005), (100.0, 8.8, 8.8, 75, 0.0012),
                           (120.0, 24.0, 15.6, 80, 0.002), (1500.0, 152.4, 230.0, 60, 0.01), (80.0, 12.3, 9.6, 70, None),
                           (3000.0, 100.0, 68.0, 65, 0.004), (60.0, 35.0, 24.0, 85, 0.003), (450.0, 50.0, 36.0, 60, None),
                           (200.0, 16.0, 13.2, 55, 0.0024), (2500.0, 210.0, 90.0, 60, 0.006), (150.0, 8.8, 13.2, 90, 0.0012),
                           (90.0, 10.26, 13.2, 75, 0.0024), (110.0, 35.0, 35.9, 70, 0.0022), (1000.0, 88.0, 57.0, 62, 0.005),
                           (75.0, 4.5, 6.3, 80, 0.0008), (5000.0, 300.0, 92.0, 60, 0.004), (130.0, 21.0, 23.5, 78, 0.0019),
                           (400.0, 40.0, 33.0, 0, 0.003), (250.0, 18.0, 17.3, 95, 0.0016)]:
        G = d / 1000 * H / (f / 1000)
        B = G * (1 - E / 100)
        inp = {"height": f"{H} m", "focal_length": f"{f} mm", "frame_length": f"{d} mm", "endlap": E}
        exp = {"result.coverage.value": G, "result.air_base.value": B, "result.base_height_ratio": B / H}
        if sp is not None:
            inp["parallax_precision"] = f"{sp} mm"
            exp["result.vertical_precision.value"] = (H / B) * (H / (f / 1000)) * (sp / 1000) * 100
        out.append((inp, exp))
    vs = [vec(i + 1, inp, exp) for i, (inp, exp) in enumerate(out)]
    n = len(vs)
    vs.append(bad(n + 1, {"height": "100 m", "focal_length": "8.8 mm", "frame_length": "8.8 mm", "endlap": 99}, "/endlap"))
    vs.append(bad(n + 2, {"height": "0 m", "focal_length": "8.8 mm", "frame_length": "8.8 mm"}, "/height"))
    return vs


TOOLS = {
    "drone.photogrammetry.photo-scale": photo_scale,
    "drone.photogrammetry.relief-displacement": relief,
    "drone.photogrammetry.parallax-height": parallax,
    "drone.photogrammetry.stereo-base": stereo,
}

if __name__ == "__main__":
    for tool in sys.argv[1:] or TOOLS:
        for v in TOOLS[tool]():
            print(json.dumps({"tool": tool, **v}, ensure_ascii=False, separators=(",", ":")))
