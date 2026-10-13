#!/usr/bin/env python3
"""Golden vectors for survey.gnss.localization.

The oracle is NumPy least squares on a full design matrix, independent of the
core's centroid-reduced closed form. Similarity rows are
[e -n 1 0; n e 0 1][a c tE tN]; affine rows solve all six coefficients.
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

SRC = "Least squares by NumPy lstsq (tools/vectors/gen_localize.py)"
VER = "NumPy 2.x"
SPEC = "add-practitioner-essentials localization scenarios"
IOGP = "IOGP Publication 373-7-2, revised September 2019, section 2.4.3.2 method 9621 worked parameters"
IOGP_VER = "Revised September 2019"
FT = 0.3048


def fit(pairs, is_affine):
    """Fit (local e, local n, grid e, grid n) tuples in meters."""
    means = [sum(p[k] for p in pairs) / len(pairs) for k in range(4)]
    centered = [tuple(p[k] - means[k] for k in range(4)) for p in pairs]
    matrix, observed = [], []
    for le, ln, ge, gn in centered:
        if is_affine:
            matrix += [[le, ln, 1, 0, 0, 0], [0, 0, 0, le, ln, 1]]
        else:
            matrix += [[le, -ln, 1, 0], [ln, le, 0, 1]]
        observed += [ge, gn]
    x, *_ = np.linalg.lstsq(np.array(matrix, float), np.array(observed, float), rcond=None)
    if is_affine:
        a, b, _, c, d, _ = x
    else:
        a, c = x[0], x[1]
        b, d = -c, a
    te = means[2] - a * means[0] - b * means[1]
    tn = means[3] - c * means[0] - d * means[1]
    residuals = [
        (gn - (c * le + d * ln + tn), ge - (a * le + b * ln + te))
        for le, ln, ge, gn in pairs
    ]
    rms = math.sqrt(sum(dn * dn + de * de for dn, de in residuals) / len(pairs))
    scale = math.sqrt(abs(a * d - b * c)) if is_affine else math.hypot(a, c)
    return {
        "scale": scale,
        "rotation": math.degrees(math.atan2(c, a)),
        "rms": rms,
        "shift_n": tn,
        "shift_e": te,
        "a": a,
        "b": b,
        "c": c,
        "d": d,
    }


def legacy_fit(pairs, is_affine):
    """Original 7-vector oracle, retained so published records reproduce."""
    means = [sum(p[k] for p in pairs) / len(pairs) for k in range(4)]
    centered = [tuple(p[k] - means[k] for k in range(4)) for p in pairs]
    matrix, observed = [], []
    for le, ln, ge, gn in centered:
        if is_affine:
            matrix += [[le, ln, 1, 0, 0, 0], [0, 0, 0, le, ln, 1]]
        else:
            matrix += [[le, -ln, 1, 0], [ln, le, 0, 1]]
        observed += [ge, gn]
    design = np.array(matrix, float)
    values = np.array(observed, float)
    x, *_ = np.linalg.lstsq(design, values, rcond=None)
    residual = values - design @ x
    rms = math.sqrt(float(np.sum(residual ** 2)) / len(pairs))
    if is_affine:
        a, b, _, c, d, _ = x
        scale = math.sqrt(abs(a * d - b * c))
    else:
        a, c = x[0], x[1]
        scale = math.hypot(a, c)
    return scale, math.degrees(math.atan2(c, a)), rms


def rows(pairs, unit="m", places=6):
    factor = FT if unit == "ft" else 1.0
    out = []
    for i, (le, ln, ge, gn) in enumerate(pairs):
        out.append({
            "name": f"P{i + 1}",
            "local_n": f"{ln / factor:.{places}f} {unit}",
            "local_e": f"{le / factor:.{places}f} {unit}",
            "grid_n": f"{gn / factor:.{places}f} {unit}",
            "grid_e": f"{ge / factor:.{places}f} {unit}",
        })
    back = [
        tuple(float(row[key].split()[0]) * factor for key in ("local_e", "local_n", "grid_e", "grid_n"))
        for row in out
    ]
    return out, back


def similarity(scale, angle_deg, shift, local, noise=None):
    angle = math.radians(angle_deg)
    out = []
    for i, (le, ln) in enumerate(local):
        dn, de = noise[i] if noise else (0.0, 0.0)
        ge = scale * (math.cos(angle) * le - math.sin(angle) * ln) + shift[0] + de
        gn = scale * (math.sin(angle) * le + math.cos(angle) * ln) + shift[1] + dn
        out.append((le, ln, ge, gn))
    return out


def affine(coefficients, shift, local, noise=None):
    a, b, c, d = coefficients
    out = []
    for i, (le, ln) in enumerate(local):
        dn, de = noise[i] if noise else (0.0, 0.0)
        out.append((le, ln, a * le + b * ln + shift[0] + de, c * le + d * ln + shift[1] + dn))
    return out


def expected(pairs, is_affine):
    solved = fit(pairs, is_affine)
    return {
        "result.scale": solved["scale"],
        "result.rotation.value": solved["rotation"],
        "result.rms.value": solved["rms"],
        "result.shift_n.value": solved["shift_n"],
        "result.shift_e.value": solved["shift_e"],
        "result.a": solved["a"],
        "result.b": solved["b"],
        "result.c": solved["c"],
        "result.d": solved["d"],
    }


def vector(index, inputs, want, source=SRC, version=VER, tolerance=None):
    want = dict(want, ok=True) if "ok" not in want else want
    tolerances = {}
    for key, value in want.items():
        if not isinstance(value, float):
            continue
        if "rotation" in key:
            tolerances[key] = {"abs": 1e-7}
        elif key.endswith(("scale", ".a", ".b", ".c", ".d")):
            tolerances[key] = {"abs": 1e-9}
        else:
            tolerances[key] = {"abs": 2e-6}
    tolerances.update(tolerance or {})
    return {
        "id": f"v{index:03d}",
        "input": inputs,
        "expect": want,
        "source": source,
        "sourceVersion": version,
        "tolerance": tolerances,
    }


def main():
    base = [(1524.0, 1524.0), (1584.96, 1828.8), (1859.28, 1645.92), (1767.84, 1432.56)]
    extra = base + [(1600.0, 1600.0), (1720.0, 1510.0), (1490.0, 1700.0), (1810.0, 1780.0)]
    old_cases = [
        (similarity(0.99997, 1.5, (956000.0, 463000.0), base, [(0.0012, -0.0009), (-0.0006, 0.0015), (0.0003, -0.0012), (-0.0009, 0.0006)]), "ft", False, 0.99997, SRC),
        (similarity(0.99992, -0.75, (500000.0, 4000000.0), base[:2]), "m", False, None, SPEC + " (two points: no redundancy)"),
        (similarity(1.00012, 12.0, (300000.0, 200000.0), base, [(0.01, 0.0), (0.0, -0.01), (-0.005, 0.005), (0.004, 0.002)]), "m", True, 1.0001, SRC),
        (similarity(1.0, 0.0, (10.0, 20.0), base[:3]), "m", True, None, SRC + " (three points: no redundancy)"),
        (similarity(0.3048, 2.0, (1000.0, 2000.0), base), "m", False, 0.99997, SPEC + " (feet entered as meters)"),
        (similarity(0.9999, 45.0, (0.0, 0.0), extra[:5]), "m", False, None, SRC),
    ]
    # These are the NumPy results published with the original 7-vector set.
    # Keep their printed floats stable across BLAS and NumPy revisions.
    published = [
        (0.9999685470724812, 1.499857059886418, 0.001241094306843177),
        (0.9999199206844521, -0.7500027466708483, 4.173493594972914e-14),
        (1.0001167915820959, 11.997753498982815, 0.002606198896818544),
        (1.0, 6.361109362927032e-15, 4.9270558744549744e-14),
        (0.3047999982688814, 2.00000363869627, 2.3089850791390984e-05),
        (0.9999001289564099, 44.999996382171176, 3.100749494252834e-05),
    ]
    output = []
    for case_index, (pairs, unit_name, is_affine, expected_scale, source) in enumerate(old_cases):
        row_values, rounded = rows(pairs, unit_name, places=4)
        # Re-evaluate as a guard that the inputs still describe the same fit,
        # then serialize the original published float spellings.
        recomputed = legacy_fit(rounded, is_affine)
        scale, rotation, rms = published[case_index]
        assert all(abs(got - want) < 1e-10 for got, want in zip(recomputed, published[case_index]))
        inputs = {"pairs": row_values}
        if is_affine:
            inputs["model"] = "affine"
        if expected_scale is not None:
            inputs["expected_scale"] = expected_scale
        want = {
            "result.scale": scale,
            "result.rotation.value": rotation,
            "result.rms.value": rms,
        }
        if len(pairs) == (3 if is_affine else 2):
            want["meta.warnings.*.code"] = "NO_REDUNDANCY"
        if expected_scale is not None and abs(scale / expected_scale - 1.0) * 1e6 > 100.0:
            want["meta.warnings.*.code"] = "SCALE_SUSPECT"
        output.append(vector(len(output) + 1, inputs, want, source, tolerance={"result.rms.value": {"abs": 1e-6}}))

    two_rows, _ = rows(old_cases[1][0], "m", places=4)
    output.append(vector(7, {"pairs": two_rows, "model": "affine"}, {"ok": False, "error.code": "INVALID_INPUT"}))

    cases = [
        (similarity(1.0004, -135.0, (-200.0, 700.0), extra[:6]), "m", False, 1.0004, SRC, VER),
        (similarity(1.0, 179.0, (12.0, -34.0), base), "m", False, None, SRC, VER),
        (similarity(1.002, 0.02, (2000.0, 1000.0), base), "m", False, None, SRC, VER),
        (similarity(0.9995, -0.03, (-500.0, 800.0), extra), "m", False, 0.9995, SRC, VER),
        (similarity(1.00001, 3.2, (1e6, -2e6), extra, [(0.002, -0.001), (-0.001, 0.002), (0.003, 0.001), (-0.002, -0.003), (0.001, 0.0), (0.0, 0.002), (-0.002, 0.001), (0.001, -0.001)]), "m", False, 1.00001, SRC, VER),
        (similarity(0.99998, 0.4, (300000.0, 400000.0), [(1e9, 1e9), (1e9 + 3.0, 1e9), (1e9, 1e9 + 4.0), (1e9 + 3.0, 1e9 + 4.0)]), "m", False, 0.99998, SRC + " (large-origin regression)", VER),
        (affine((1.0002, 0.0004, -0.0003, 0.9998), (600.0, -900.0), base), "m", True, None, SRC, VER),
        (affine((1.02, 0.03, -0.01, 0.98), (-20.0, 30.0), extra[:6]), "m", True, None, SRC, VER),
        (affine((-1.0, 0.02, 0.01, 1.0), (300.0, 400.0), base), "m", True, None, SRC, VER),
        (affine((0.9998, -0.0007, 0.0005, 1.0003), (7e5, 3e6), extra, [(0.002, -0.003), (0.001, 0.002), (-0.002, 0.001), (0.003, -0.001), (-0.001, -0.002), (0.0, 0.003), (0.002, 0.0), (-0.003, 0.001)]), "ft", True, 1.0, SRC, VER),
        (similarity(1.0, 0.0, (0.0, 0.0), [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)]), "m", False, 1.0, SRC, VER),
    ]

    # Method 9621 publishes the parameters and one transformed point. Four
    # controls generated from those parameters make the fit itself testable;
    # the first is the source's 300000, 4500000 point.
    theta = math.radians(1.56504 / 3600.0)
    iogp_local = [(300000.0, 4500000.0), (301000.0, 4500000.0), (300000.0, 4501000.0), (301000.0, 4501000.0)]
    iogp_pairs = []
    for x, y in iogp_local:
        ge = -129.549 + 1.00000155 * (x * math.cos(theta) + y * math.sin(theta))
        gn = -208.185 + 1.00000155 * (-x * math.sin(theta) + y * math.cos(theta))
        iogp_pairs.append((x, y, ge, gn))
    cases.append((iogp_pairs, "m", False, 1.00000155, IOGP, IOGP_VER))

    for pairs, unit_name, is_affine, expected_scale, source, source_version in cases:
        row_values, rounded = rows(pairs, unit_name)
        inputs = {"pairs": row_values}
        if is_affine:
            inputs["model"] = "affine"
        if expected_scale is not None:
            inputs["expected_scale"] = expected_scale
        want = expected(rounded, is_affine)
        minimum = 3 if is_affine else 2
        if len(pairs) == minimum:
            want["meta.warnings.*.code"] = "NO_REDUNDANCY"
        if expected_scale is not None and abs(want["result.scale"] / expected_scale - 1.0) * 1e6 > 100.0:
            want["meta.warnings.*.code"] = "SCALE_SUSPECT"
        output.append(vector(len(output) + 1, inputs, want, source, source_version))

    mixed_pairs = similarity(0.99995, 0.25, (400000.0, 900000.0), base)
    mixed_rows, _ = rows(mixed_pairs)
    for index in (1, 3):
        for key in ("local_n", "local_e", "grid_n", "grid_e"):
            meters = float(mixed_rows[index][key].split()[0])
            mixed_rows[index][key] = f"{meters / FT:.9f} ft"
    reread = []
    for row in mixed_rows:
        values = []
        for key in ("local_e", "local_n", "grid_e", "grid_n"):
            value, unit_name = row[key].split()
            values.append(float(value) * (FT if unit_name == "ft" else 1.0))
        reread.append(tuple(values))
    output.append(vector(len(output) + 1, {"pairs": mixed_rows, "expected_scale": 0.99995}, expected(reread, False)))

    failures = [
        ({"pairs": rows([(1.0, 2.0, 3.0, 4.0), (1.0, 2.0, 8.0, 9.0)])[0]}, "DEGENERATE_GEOMETRY", "/pairs"),
        ({"pairs": rows([(0.0, 0.0, 0.0, 0.0), (1.0, 1.0, 2.0, 2.0), (2.0, 2.0, 4.0, 4.0)])[0], "model": "affine"}, "DEGENERATE_GEOMETRY", "/pairs"),
        ({"pairs": rows(old_cases[0][0])[0], "expected_scale": 0.0}, "INVALID_INPUT", "/expected_scale"),
        ({"pairs": rows(old_cases[0][0])[0], "expected_scale": -1.0}, "INVALID_INPUT", "/expected_scale"),
    ]
    for inputs, code, field in failures:
        want = {"ok": False, "error.code": code}
        if field:
            want["error.field"] = field
        output.append(vector(len(output) + 1, inputs, want, SPEC))

    destination = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    path = destination / "survey.gnss.localization.jsonl"
    path.write_text("".join(json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n" for item in output))


if __name__ == "__main__":
    main()
