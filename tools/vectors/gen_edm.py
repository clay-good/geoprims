#!/usr/bin/env python3
"""Golden vectors for the EDM correction, worked by hand: the IAG 1999 closed
formula N_G = 287.6155 + 4.88660/l^2 + 0.06800/l^4 and N_L = (273.15/1013.25)
N_G p/T - 11.27 e/T (Landgate calibration manual, section 4.2.3), ppm = N_ref -
N_L, and corrected = D (1 + ppm 1e-6) + prism constant."""
import json
import math
import sys
from pathlib import Path

SRC = "IAG 1999 closed formula worked in Python (tools/vectors/gen_edm.py)"
SPEC = "add-survey-suite ppm scenario"
TRIMBLE = "Trimble Access Calculations Reference Guide, revision D, pages 9-10"


def vec(i, inp, exp, src=SRC, tol=1e-9, version="2026"):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": version, "tolerance": t}


def es(t):
    return 6.1094 * math.exp(17.625 * t / (t + 243.04))


def cases():
    out = []
    for d, ppm, prism in [(1000, 12, -30), (523.456, -4.5, 0), (2500, 0, 17.5)]:
        inp = {"distance": f"{d} m", "ppm": ppm}
        if prism:
            inp["prism_constant"] = f"{prism} mm"
        atm = ppm * 1e-6 * d
        out.append(vec(len(out) + 1, inp, {"result.corrected.value": d + atm + prism / 1000, "result.ppm": ppm,
                                           "result.atmospheric_correction.value": atm, "result.prism_correction.value": prism / 1000}))
    out[0]["source"] = SPEC + " (1,000.000 + 0.012 - 0.030 = 999.982 m)"
    for d, t, p, rh, lam, nref in [(1000, 28, 980, 60, 0.658, 286.34), (1500, -5, 1030, None, 0.658, 286.34), (800, 35, 850, 30, 0.870, 283.0)]:
        ng = 287.6155 + 4.88660 / lam ** 2 + 0.06800 / lam ** 4
        e = rh / 100 * es(t) if rh is not None else 0.0
        nl = (273.15 / 1013.25) * ng * p / (273.15 + t) - 11.27 * e / (273.15 + t)
        ppm = nref - nl
        inp = {"distance": f"{d} m", "temperature": f"{t} degC", "pressure": f"{p} hPa", "wavelength": lam, "reference_refractivity": nref}
        if rh is not None:
            inp["humidity"] = rh
        out.append(vec(len(out) + 1, inp, {"result.ppm": ppm, "result.refractivity": nl, "result.corrected.value": d + ppm * 1e-6 * d}))
    for inp in [{"distance": "1000 m"}, {"distance": "1000 m", "ppm": 5, "temperature": "20 degC", "pressure": "1000 hPa"},
                {"distance": "1000 m", "temperature": "20 degC", "pressure": "1000 hPa", "wavelength": 0.658}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))

    # Trimble Access publishes this formula and these J/N pairs for the three
    # named Trimble instrument families in its current calculations guide.
    presets = {
        "trimble-access-5600": (274.41, 79.39),
        "trimble-access-3300-3600": (278.77, 80.653),
        "trimble-access-tts300-500": (270.0, 79.167),
    }
    conditions = [(1000, 20, 1013.25, 0), (750, -20, 900, -30), (1500, 35, 850, 17.5), (250, 0, 1100, 0)]
    for preset, (j, n) in presets.items():
        for d, t, p, prism in conditions:
            ppm = j - n * p / (273.16 + t)
            atm = ppm * 1e-6 * d
            inp = {"distance": f"{d} m", "temperature": f"{t} degC", "pressure": f"{p} hPa",
                   "instrument_formula": preset}
            if prism:
                inp["prism_constant"] = f"{prism} mm"
            out.append(vec(len(out) + 1, inp,
                           {"result.ppm": ppm, "result.atmospheric_correction.value": atm,
                            "result.prism_correction.value": prism / 1000,
                            "result.corrected.value": d + atm + prism / 1000},
                           TRIMBLE, version="Revision D (April 2024)"))

    invalid = [
        {"distance": "1000 m", "ppm": 5, "instrument_formula": "trimble-access-5600"},
        {"distance": "1000 m", "temperature": "20 degC", "pressure": "1000 hPa",
         "humidity": 50, "instrument_formula": "trimble-access-5600"},
        {"distance": "1000 m", "temperature": "20 degC", "pressure": "1000 hPa",
         "wavelength": 0.658, "instrument_formula": "trimble-access-5600"},
        {"distance": "1000 m", "temperature": "20 degC", "instrument_formula": "trimble-access-5600"},
    ]
    for inp in invalid:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"},
                       TRIMBLE, version="Revision D (April 2024)"))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "survey.reduction.edm-correction.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases()))


if __name__ == "__main__":
    main()
