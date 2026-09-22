#!/usr/bin/env python3
"""Golden vectors for dataset size, thermal footprint, and the link budget,
worked by hand: orthomosaic bytes = area / GSD^2 x bands x bits / 8; LAS =
375 + points x the LAS 1.4 R15 record size; footprint = IFOV x distance with
the 3 x 3 rule; FSPL = 20 log d_km + 20 log f_MHz + 32.44 (ITU-R P.525)."""
import json
import math
import sys
from pathlib import Path

SRC = "Worked by hand in Python (tools/vectors/gen_sensing.py)"
SPEC = "add-practitioner-essentials scenarios"
PDRF = [20, 28, 26, 34, 57, 63, 30, 36, 38, 59, 67]


def vec(i, inp, exp, src=SRC, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": tol, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def dataset():
    out = []
    g = 7.5
    out.append(vec(1, {"area": "1 km2", "gsd": "2 cm"}, {"result.ortho_gb": g, "result.ortho_low_gb": g / 2, "result.ortho_high_gb": g / 10}, SPEC + " (orthomosaic: 7.5 GB uncompressed)"))
    a, gsd, bands, bits = 0.25e6, 0.015, 4, 16
    gb = a / gsd ** 2 * bands * bits / 8 / 1e9
    out.append(vec(2, {"area": "0.25 km2", "gsd": "1.5 cm", "bands": 4, "bit_depth": 16, "compression_low": 1.5, "compression_high": 4},
                   {"result.ortho_gb": gb, "result.ortho_low_gb": gb / 1.5, "result.ortho_high_gb": gb / 4}))
    out.append(vec(3, {"image_count": 850, "image_mb": 25}, {"result.raw_gb": 850 * 25 / 1000}))
    for pts, fmt, ratio in [(250e6, None, None), (1e9, 1, 10)]:
        rec = PDRF[fmt if fmt is not None else 6]
        las = (375 + pts * rec) / 1e9
        inp = {"point_count": pts}
        if fmt is not None:
            inp.update({"point_format": fmt, "laz_ratio": ratio})
        out.append(vec(len(out) + 1, inp, {"result.las_gb": las, "result.laz_gb": las / (ratio or 7)}))
    for inp in [{}, {"area": "1 km2"}, {"image_count": 10}, {"area": "1 km2", "gsd": "2 cm", "compression_low": 5, "compression_high": 2}]:
        out.append(vec(len(out) + 1, inp, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def thermal():
    out = []
    for ifov, d, target in [(1.3, 30, None), (1.3, 30, 150), (0.68, 60, 100), (2.2, 12, 40)]:
        foot = ifov / 1000 * d
        inp = {"ifov": f"{ifov} mrad", "distance": f"{d} m"}
        exp = {"result.footprint.value": foot * 1000, "result.min_target.value": 3 * foot * 1000}
        if target:
            inp["target_size"] = f"{target} mm"
            exp["result.max_distance.value"] = target / 1000 / (3 * ifov / 1000)
        out.append(vec(len(out) + 1, inp, exp))
    out[0]["source"] = SPEC + " (solar panel: 39 mm footprint, 117 mm by the 3 x 3 rule)"
    out.append(vec(len(out) + 1, {"pixel_pitch": "12 um", "focal_length": "9 mm", "distance": "50 m", "pixels": 5},
                   {"result.footprint.value": 12e-6 / 9e-3 * 50 * 1000, "result.min_target.value": 5 * 12e-6 / 9e-3 * 50 * 1000}))
    out.append(vec(len(out) + 1, {"distance": "30 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def link():
    out = []

    def fspl(f, d):
        return 20 * math.log10(d) + 20 * math.log10(f) + 32.44

    L = fspl(2400, 5)
    out.append(vec(1, {"frequency": "2400 MHz", "distance": "5 km"}, {"result.fspl": L}, SPEC + " (2.4 GHz at 5 km: about 114.0 dB)"))
    rx = 20 + 2 + 2 - L
    out.append(vec(2, {"frequency": "2400 MHz", "distance": "5 km", "tx_power": 20, "rx_sensitivity": -90, "tx_gain": 2, "rx_gain": 2, "jurisdiction": "eu"},
                   {"result.fspl": L, "result.rx_power": rx, "result.fade_margin": rx + 90, "result.eirp": 22.0,
                    "result.eirp_status": "Beyond your 20 dBm EIRP limit (ETSI EN 300 328 V2.2.2 (2019-07), clauses 4.3.1.2.3 and 4.3.2.2.3, rules as of 2026-09-22)"}))
    L2 = fspl(2440, 2)
    out.append(vec(3, {"frequency": "2.44 GHz", "distance": "2 km", "tx_power": 27, "rx_sensitivity": -95, "tx_gain": 3, "cable_loss": 2, "jurisdiction": "us"},
                   {"result.fspl": L2, "result.rx_power": 27 + 3 - 2 - L2, "result.eirp": 29.0,
                    "result.eirp_status": "Within your 36 dBm EIRP limit (47 CFR 15.247(b)(3) and (b)(4), rules as of 2026-09-22)"}))
    L3 = fspl(5800, 1.5)
    out.append(vec(4, {"frequency": "5800 MHz", "distance": "1.5 km", "tx_power": 20, "jurisdiction": "eu"},
                   {"result.fspl": L3, "result.eirp_status": "No EIRP limit on file for this band; check the rules for your frequency"}))
    out.append(vec(5, {"frequency": "2400 MHz", "distance": "0 km"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def lidar():
    out = []
    for prr, fov, h, v, ov, ret in [(240000, 70, 100, 10, 0, 1), (240000, 70, 100, 10, 30, 1.5), (100000, 60, 120, 15, 50, 1),
                                    (50000, 90, 300, 30, 20, 2), (20000, 40, 250, 25, 0, 1)]:
        swath = 2 * h * math.tan(math.radians(fov / 2))
        spacing = swath * (1 - ov / 100)
        one, agg = prr / (v * swath), prr / (v * spacing)
        ql = next((f"{q} (at least {m:g} pulses per m²)" for q, m in [("QL1", 8.0), ("QL2", 2.0), ("QL3", 0.5)] if agg >= m), "below QL3 (0.5 pulses per m²)")
        inp = {"pulse_rate": f"{prr / 1000:g} kHz", "fov": f"{fov} deg", "height": f"{h} m", "speed": f"{v} m/s"}
        if ov:
            inp["side_overlap"] = ov
        if ret != 1:
            inp["returns"] = ret
        out.append(vec(len(out) + 1, inp, {"result.swath.value": swath, "result.line_spacing.value": spacing, "result.pulse_density": one,
                                           "result.aggregate_density": agg, "result.point_density": agg * ret, "result.quality_level": ql}))
    out[0]["source"] = SPEC + " (swath about 140.0 m, 171 pulses per m², beyond QL1)"
    out.append(vec(len(out) + 1, {"pulse_rate": "240 kHz", "fov": "180 deg", "height": "100 m", "speed": "10 m/s"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("drone.sensors.dataset-size", dataset()), ("drone.sensors.thermal-footprint", thermal()), ("drone.links.link-budget", link()), ("drone.sensors.lidar-plan", lidar())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
