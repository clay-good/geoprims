#!/usr/bin/env python3
"""Golden vectors for navigation.vessel.under-keel-clearance
(add-marine-navigation, navigation/under-keel-clearance).

The stack is arithmetic, so the vectors come from a second implementation of
it here, in Python, independent of the Rust one: clearance = charted depth +
tide - (draft + squat + margin).

Squat follows Barrass as the "Mircea cel Batran" Naval Academy Scientific
Bulletin XIX(1) 2016 (Deleanu and Panaitescu, CC BY-NC-SA) prints it:

    S_max = K C_B V_K^2 / 100        (their equation 2, V_K in knots)
    K     = 5.74 S^0.76              (their equation 3, 0.10 <= S <= 0.25)
    other end = [1 - 40 (0.7 - C_B)^2] S_max   (their equation 4)

with Table 1's published ranges: C_B 0.5 to 0.85 and h/T 1.1 to 1.4. S = 0.10
is a wide river (K = 1.0) and S = 0.25 a restricted channel (K = 2.0).

    gen_ukc.py    # print the vectors as JSON lines

Writes nothing: new vectors are appended by hand so published files stay
frozen.
"""
import json

PAPER = ("\"Mircea cel Batran\" Naval Academy Scientific Bulletin XIX(1) 2016, "
         "pp. 105-111, equations (2) to (4) and Table 1, reimplemented in Python")
SPEC = "add-marine-navigation navigation/under-keel-clearance scenario"
S_OPEN, S_CONFINED = 0.10, 0.25


def k_of(s):
    return 5.74 * s**0.76


def squat(cb, v, s=S_OPEN):
    return k_of(s) * cb * v * v / 100.0


def other_end(cb, smax):
    return (1 - 40 * (0.7 - cb) ** 2) * smax


def tol(fields, t=1e-9):
    return {f: {"abs": t} for f in fields}


rows = []


def case(inp, expect, source, tolerance):
    rows.append({"input": inp, "expect": expect, "source": source,
                 "sourceVersion": "Python math; the 2016 bulletin", "tolerance": tolerance})


def stack_case(depth, tide, draft, sq, margin, source, extra_input=None, warn=None):
    water = depth + tide
    under = water - (draft + sq)
    clear = under - margin
    inp = {"charted_depth": f"{depth} m", "tide": f"{tide} m", "draft": f"{draft} m",
           "squat": f"{sq} m", "margin": f"{margin} m"}
    if extra_input:
        inp.update(extra_input)
    ex = {"ok": True, "result.clearance.value": clear, "result.under_keel.value": under,
          "result.water_depth.value": water, "result.keel_depth.value": draft + sq}
    if warn:
        ex["meta.warnings.*.code"] = warn
    case(inp, ex, source, tol([k for k in ex if k.startswith("result.")]))


# The spec's own stack, with squat given.
stack_case(12.0, 1.5, 10.8, 0.6, 1.0, SPEC + ": the Stack scenario")
# Negative clearance carries BELOW_MARGIN.
stack_case(9.0, 0.4, 8.9, 0.5, 1.0, SPEC + ": the Negative clearance scenario", warn="BELOW_MARGIN")
# More stacks, each a second run of the same arithmetic.
for depth, tide, draft, sq, margin in [
    (15.0, 0.0, 12.0, 0.8, 1.5), (11.5, 2.3, 10.4, 0.45, 0.6), (20.0, -0.5, 14.2, 1.1, 2.0),
    (8.2, 1.1, 7.0, 0.3, 0.5), (13.75, 0.85, 11.9, 0.55, 1.25), (30.0, 1.0, 22.5, 1.4, 3.0),
]:
    stack_case(depth, tide, draft, sq, margin, "Python arithmetic of the same stack")

# Barrass squat, open water (K = 1 at S = 0.10), inside the published ranges.
for cb, v, draft, water in [(0.80, 10.0, 12.0, 14.4), (0.75, 8.0, 9.0, 11.0),
                            (0.85, 12.0, 15.0, 18.0), (0.50, 14.0, 7.0, 8.4),
                            (0.667, 8.0, 7.76, 9.7)]:
    sq = squat(cb, v)
    depth, tide, margin = water, 0.0, 1.0
    under = water - (draft + sq)
    ex = {"ok": True, "result.squat_used.value": sq, "result.other_end_check": None,
          "result.under_keel.value": under, "result.clearance.value": under - margin,
          "result.squat_other_end.value": other_end(cb, sq)}
    del ex["result.other_end_check"]
    if under - margin < 0:
        ex["meta.warnings.*.code"] = "BELOW_MARGIN"
    case({"charted_depth": f"{depth} m", "tide": "0 m", "draft": f"{draft} m",
          "margin": f"{margin} m", "speed": f"{v} kt", "block_coefficient": cb,
          "channel": "open-water"},
         ex, PAPER + ": open water, K = 1", tol([k for k in ex if k.startswith("result.")], 1e-9))

# A confined channel: S = 0.25, K = 2.
for cb, v, draft, water in [(0.80, 8.0, 10.0, 12.0), (0.70, 6.0, 9.5, 11.5), (0.62, 9.0, 8.0, 10.0)]:
    sq = squat(cb, v, S_CONFINED)
    under = water - (draft + sq)
    ex = {"ok": True, "result.squat_used.value": sq, "result.under_keel.value": under,
          "result.clearance.value": under - 0.5}
    if under - 0.5 < 0:
        ex["meta.warnings.*.code"] = "BELOW_MARGIN"
    case({"charted_depth": f"{water} m", "tide": "0 m", "draft": f"{draft} m", "margin": "0.5 m",
          "speed": f"{v} kt", "block_coefficient": cb, "channel": "confined-channel"},
         ex, PAPER + ": a confined channel, S = 0.25", tol([k for k in ex if k.startswith("result.")]))

# A blockage factor worked out from the channel, including the 2016 paper's
# own cargo ship: W = 123 m, h = 10 m, B = 17.1 m, T = 7.76 m, C_B = 0.667,
# 8 knots, so A_S = 0.98 B T = 130.0421 m2, A_C = W h = 1230 m2, S = 0.105725.
for cb, v, draft, water, s in [(0.667, 8.0, 7.76, 10.0, 130.0421 / 1230),
                               (0.80, 7.0, 11.0, 13.0, 0.18), (0.72, 10.0, 9.0, 11.7, 0.22),
                               (0.55, 11.0, 8.5, 10.5, 0.12)]:
    s = round(s, 6)  # the vector carries the rounded factor, so compute with it
    sq = squat(cb, v, s)
    under = water - (draft + sq)
    ex = {"ok": True, "result.squat_used.value": sq, "result.under_keel.value": under,
          "result.squat_other_end.value": other_end(cb, sq)}
    case({"charted_depth": f"{water} m", "tide": "0 m", "draft": f"{draft} m", "margin": "0.3 m",
          "speed": f"{v} kt", "block_coefficient": cb, "channel": "blockage-factor",
          "blockage_factor": round(s, 6)},
         ex, PAPER + f": blockage factor {s:.6f}", tol([k for k in ex if k.startswith("result.")]))

# Refusals, each naming its published range.
case({"charted_depth": "20.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
      "speed": "10 kt", "block_coefficient": 0.8},
     {"ok": False, "error.code": "OUT_OF_DOMAIN"},
     SPEC + ": the Out of range scenario, h/T = 2.0 against the published 1.1 to 1.4", {})
case({"charted_depth": "12.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
      "speed": "10 kt", "block_coefficient": 0.95},
     {"ok": False, "error.code": "OUT_OF_DOMAIN"},
     PAPER + ": Table 1 holds Barrass to C_B 0.5 to 0.85", {})
case({"charted_depth": "12.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
      "speed": "10 kt", "block_coefficient": 0.8, "channel": "blockage-factor",
      "blockage_factor": 0.05},
     {"ok": False, "error.code": "OUT_OF_DOMAIN"},
     PAPER + ": K = 5.74 S^0.76 is published for S from 0.10 to 0.25", {})
case({"charted_depth": "12.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
      "speed": "10 kt"},
     {"ok": False, "error.code": "INVALID_INPUT"},
     SPEC + ": the squat estimate needs the block coefficient", {})

for i, r in enumerate(rows, 1):
    print(json.dumps({"id": f"v{i:03d}", **r}, ensure_ascii=False, separators=(",", ":")))
