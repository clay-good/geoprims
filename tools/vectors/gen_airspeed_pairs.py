#!/usr/bin/env python3
"""Golden vectors for the allow-listed airspeed pairs (aviation.airspeed.ias-to-tas
and the rest). Each pair is its parent tool with a narrower form, so its
vectors come from the same independent Python implementation of the Gracey
(NASA RP-1046) relations as the parents' (tools/vectors/gen_aviation.py),
plus linear interpolation in the calibration table for indicated airspeed.

Usage: python3 tools/vectors/gen_airspeed_pairs.py
"""
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_aviation import A0, AS_SRC, AS_VER, FT, KT, P0, R, cas_from_mach, isa, mach_for, qc_ratio, vec  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "core/vectors"
TABLE = [(60, 63), (100, 101), (140, 139)]
CAL = [{"indicated": f"{i} kt", "calibrated": f"{c} kt"} for i, c in TABLE]


def calibrated(ias):
    for (x0, y0), (x1, y1) in zip(TABLE, TABLE[1:]):
        if x0 <= ias <= x1:
            return y0 + (y1 - y0) * (ias - x0) / (x1 - x0)
    raise ValueError(ias)


def at_cas(cas, ft, oat):
    p = isa(ft * FT)[1]
    m = mach_for(P0 * qc_ratio(cas * KT / A0) / p)
    out = {"result.cas.value": cas, "result.mach": m, "result.eas.value": m * A0 * math.sqrt(p / P0) / KT}
    if oat is not None:
        out["result.tas.value"] = m * math.sqrt(1.4 * R * (oat + 273.15)) / KT
    return out


def at_mach(m, ft, oat):
    p = isa(ft * FT)[1]
    out = {"result.cas.value": cas_from_mach(m, p), "result.mach": m, "result.eas.value": m * A0 * math.sqrt(p / P0) / KT}
    if oat is not None:
        out["result.tas.value"] = m * math.sqrt(1.4 * R * (oat + 273.15)) / KT
    return out


def ias_vectors():
    out = []
    for ias, ft, oat in [(120, 8000, 0), (75, 3000, 20), (138, 12000, -10), (60, 0, 15), (100, 16000, -20)]:
        inp = {"airspeed": f"{ias} kt", "pressure_altitude": f"{ft} ft", "temperature": f"{oat} degC", "calibration": CAL}
        out.append(vec(len(out) + 1, inp, at_cas(calibrated(ias), ft, oat), 1e-9, AS_SRC, AS_VER))
    return out


def eas_vectors(with_temperature):
    out = []
    for eas, ft, oat in [(248.1, 10000, -5), (300, 35000, -54.3), (150, 2000, 12), (420, 25000, -35), (90, -1000, 25)]:
        p = isa(ft * FT)[1]
        m = eas * KT / (A0 * math.sqrt(p / P0))
        inp = {"eas": f"{eas} kt", "pressure_altitude": f"{ft} ft"}
        if with_temperature:
            inp["temperature"] = f"{oat} degC"
        out.append(vec(len(out) + 1, inp, at_mach(m, ft, oat if with_temperature else None), 1e-9, AS_SRC, AS_VER))
    return out


def mach_vectors():
    return [vec(i, {"mach": m, "pressure_altitude": f"{ft} ft"}, at_mach(m, ft, None), 1e-9, AS_SRC, AS_VER)
            for i, (m, ft) in enumerate([(0.78, 35000), (0.5, 20000), (0.85, 41000), (1.6, 45000), (0.2, 0)], 1)]


def tas_vectors():
    out = []
    for tas, ft, oat in [(288.6, 10000, -5), (450, 35000, -54.3), (110, 0, 15), (520, 41000, -56.5), (60, 5000, 30)]:
        m = tas * KT / math.sqrt(1.4 * R * (oat + 273.15))
        inp = {"tas": f"{tas} kt", "pressure_altitude": f"{ft} ft", "temperature": f"{oat} degC"}
        out.append(vec(len(out) + 1, inp, at_mach(m, ft, oat), 1e-9, AS_SRC, AS_VER))
    return out


FILES = {
    "ias-to-tas": ias_vectors(),
    "ias-to-mach": ias_vectors(),
    "ias-to-eas": ias_vectors(),
    "eas-to-tas": eas_vectors(True),
    "eas-to-mach": eas_vectors(False),
    "mach-to-eas": mach_vectors(),
    "tas-to-eas": tas_vectors(),
}
for slug, vs in FILES.items():
    path = OUT / f"aviation.airspeed.{slug}.jsonl"
    path.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))
print(f"wrote {sum(map(len, FILES.values()))} vectors in {len(FILES)} files")
