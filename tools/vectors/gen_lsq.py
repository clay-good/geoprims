#!/usr/bin/env python3
"""Golden vectors for survey.cogo.least-squares-2d from an independent
weighted least-squares solver: numerical (central-difference) Jacobians,
NumPy normal equations, SciPy chi-square and F quantiles, and error ellipses
from the eigenvectors of each point's covariance block. Networks in meters."""
import json
import math
import sys
from pathlib import Path

import numpy as np
from scipy.stats import chi2, f as fdist

SRC, VER = "Weighted least squares with numerical Jacobians, NumPy and SciPy (tools/vectors/gen_lsq.py)", "NumPy 2.x, SciPy 1.x"
SPEC = "add-survey-suite least-squares scenarios"
AS = math.pi / 648000


def dms(deg):
    deg %= 360
    d = int(deg)
    m = int((deg - d) * 60)
    s = (deg - d - m / 60) * 3600
    return f"{d}-{m:02d}-{s:05.2f}"


def solve(control, unknowns, dists, angles, dirs):
    names = [n for n, _, _ in unknowns]
    sets = []
    for st, _, _, _ in dirs:
        if st not in sets:
            sets.append(st)
    x = np.array([v for _, n, e in unknowns for v in (e, n)] + [0.0] * len(sets), float)
    ctl = {n: (e, nn) for n, nn, e in control}

    def pos(p, x):
        if p in ctl:
            return ctl[p]
        i = names.index(p)
        return x[2 * i], x[2 * i + 1]

    def az(a, b, x):
        pa, pb = pos(a, x), pos(b, x)
        return math.atan2(pb[0] - pa[0], pb[1] - pa[1])

    def model(x):
        out = []
        for a, b, _, _ in dists:
            pa, pb = pos(a, x), pos(b, x)
            out.append(math.hypot(pb[0] - pa[0], pb[1] - pa[1]))
        for b, s, f, _, _ in angles:
            out.append((az(s, f, x) - az(s, b, x)) % (2 * math.pi))
        for st, t, _, _ in dirs:
            out.append((az(st, t, x) - x[2 * len(names) + sets.index(st)]) % (2 * math.pi))
        return np.array(out)

    obs = np.array([d for _, _, d, _ in dists] + [math.radians(a) for *_, a, _ in angles] + [math.radians(v) for _, _, v, _ in dirs])
    sd = np.array([s for *_, s in dists] + [s * AS for *_, s in angles] + [s * AS for *_, s in dirs])
    w = np.diag(1 / sd ** 2)
    for k, st in enumerate(sets):
        t, v = next((t, v) for s2, t, v, _ in dirs if s2 == st)
        x[2 * len(names) + k] = az(st, t, x) - math.radians(v)
    wrap = lambda r: (r + math.pi) % (2 * math.pi) - math.pi
    nd = len(dists)
    for _ in range(30):
        f0 = model(x)
        l = obs - f0
        l[nd:] = [wrap(v) for v in l[nd:]]
        J = np.zeros((len(obs), len(x)))
        for j in range(len(x)):
            h = 1e-4 if j < 2 * len(names) else 1e-9
            xp, xm = x.copy(), x.copy()
            xp[j] += h
            xm[j] -= h
            d = model(xp) - model(xm)
            d[nd:] = [wrap(v) for v in d[nd:]]
            J[:, j] = d / (2 * h)
        N = J.T @ w @ J
        dx = np.linalg.solve(N, J.T @ w @ l)
        x += dx
        if np.max(np.abs(dx[:2 * len(names)])) < 1e-8:
            break
    v = model(x) - obs
    v[nd:] = [wrap(r) for r in v[nd:]]
    vtwv = float(v @ w @ v)
    r = len(obs) - len(x)
    Q = np.linalg.inv(N)
    s0 = math.sqrt(vtwv / r) if r > 0 else 1.0
    c = math.sqrt(2 * fdist.ppf(0.95, 2, r)) if r > 0 else math.sqrt(chi2.ppf(0.95, 2))
    pts = []
    for i, n in enumerate(names):
        blk = Q[2 * i:2 * i + 2, 2 * i:2 * i + 2]  # [E, N]
        vals, vecs = np.linalg.eigh(blk)
        major = vecs[:, 1]
        orient = math.degrees(math.atan2(major[0], major[1])) % 180
        pts.append({"northing": x[2 * i + 1], "easting": x[2 * i], "semi_major": c * s0 * math.sqrt(vals[1]),
                    "semi_minor": c * s0 * math.sqrt(max(vals[0], 0)), "orientation": orient,
                    "sd_northing": s0 * math.sqrt(blk[1, 1]), "sd_easting": s0 * math.sqrt(blk[0, 0])})
    test = "not possible without redundancy" if r == 0 else ("passed" if chi2.ppf(0.025, r) <= vtwv <= chi2.ppf(0.975, r) else "failed")
    return {"r": r, "s02": vtwv / r if r else None, "vtwv": vtwv, "test": test, "points": pts}


def net_inputs(control, unknowns, dists, angles, dirs):
    m = lambda v: f"{v:.4f} m"
    inp = {"control": [{"name": n, "northing": m(nn), "easting": m(e)} for n, nn, e in control],
           "unknowns": [{"name": n, "northing": m(nn), "easting": m(e)} for n, nn, e in unknowns]}
    if dists:
        inp["distances"] = [{"from": a, "to": b, "distance": m(d), "sd": m(s)} for a, b, d, s in dists]
    if angles:
        inp["angles"] = [{"backsight": b, "station": s, "foresight": f, "angle": dms(a), "sd": sd} for b, s, f, a, sd in angles]
    if dirs:
        inp["directions"] = [{"station": s, "target": t, "direction": dms(v), "sd": sd} for s, t, v, sd in dirs]
    return inp


def parse_dms(text):
    d, m, s = text.split("-")
    return int(d) + int(m) / 60 + float(s) / 3600


def main():
    ctl2 = [("A", 1000.0, 1000.0), ("B", 1000.0, 1400.0)]
    nets = []
    # 1. The tool's example: one point from two control points.
    nets.append((ctl2, [("P", 1300.0, 1200.0)], [("A", "P", 360.567, 0.005), ("B", "P", 360.551, 0.005)],
                 [("B", "A", "P", parse_dms("303-41-26"), 5), ("P", "B", "A", parse_dms("303-41-20"), 5)], [], SRC))
    # 2. A braced quadrilateral: two new points, all distances and some angles.
    truth = {"A": (1000, 1000), "B": (1400, 1000), "C": (1420, 1350), "D": (980, 1330)}  # E, N
    d = lambda a, b: math.hypot(truth[b][0] - truth[a][0], truth[b][1] - truth[a][1])
    azd = lambda a, b: math.degrees(math.atan2(truth[b][0] - truth[a][0], truth[b][1] - truth[a][1]))
    ang = lambda b, s, f: (azd(s, f) - azd(s, b)) % 360
    noise = iter([0.004, -0.003, 0.002, -0.005, 0.003, -0.002, 0.001])
    ctl = [("A", 1000.0, 1000.0), ("B", 1000.0, 1400.0)]
    unk = [("C", 1352.0, 1418.0), ("D", 1328.0, 982.0)]
    dists = [(a, b, d(a, b) + next(noise), 0.005) for a, b in [("A", "C"), ("A", "D"), ("B", "C"), ("B", "D"), ("C", "D")]]
    angles = [("B", "A", "C", ang("B", "A", "C") + 3 / 3600, 5), ("A", "B", "D", ang("A", "B", "D") - 4 / 3600, 5),
              ("D", "C", "A", ang("D", "C", "A") + 2 / 3600, 5)]
    nets.append((ctl, unk, dists, angles, [], SRC))
    # 3. Direction sets at A and B, with distances.
    dirs = [("A", "B", (azd("A", "B") - 17.5) % 360 + 2 / 3600, 3), ("A", "C", (azd("A", "C") - 17.5) % 360 - 1 / 3600, 3), ("A", "D", (azd("A", "D") - 17.5) % 360, 3),
            ("B", "A", (azd("B", "A") + 40.25) % 360 - 2 / 3600, 3), ("B", "C", (azd("B", "C") + 40.25) % 360 + 3 / 3600, 3), ("B", "D", (azd("B", "D") + 40.25) % 360 + 1 / 3600, 3)]
    nets.append((ctl, unk, dists[:3], [], dirs, SRC))
    # 4. The chi-square failure: the same network claimed 10 times more precise.
    nets.append((ctl, unk, [(a, b, v, 0.0005) for a, b, v, _ in dists], [(b, s, f, a, 0.5) for b, s, f, a, _ in angles], [], SPEC + " (chi-square failure)"))
    # 5. No redundancy: two distances fix one point exactly.
    nets.append((ctl2, [("P", 1300.0, 1200.0)], [("A", "P", 360.555, 0.005), ("B", "P", 360.555, 0.005)], [], [], SRC))
    out = []
    for i, (c, u, ds, an, di, src) in enumerate(nets, 1):
        res = solve(c, u, ds, an, di)
        inp = net_inputs(c, u, ds, an, di)
        # Angles go in as the text the tool reads, so solve on exactly that.
        if an:
            an2 = [(b, s, f, parse_dms(a["angle"]), sd) for (b, s, f, _, sd), a in zip(an, inp["angles"])]
            ds2 = [(a, b, float(x["distance"].split()[0]), s) for (a, b, _, s), x in zip(ds, inp.get("distances", []))]
            res = solve(c, [(n, float(r["northing"].split()[0]), float(r["easting"].split()[0])) for (n, _, _), r in zip(u, inp["unknowns"])], ds2, an2,
                        [(s, t, parse_dms(x["direction"]), sd) for (s, t, _, sd), x in zip(di, inp.get("directions", []))])
        elif di:
            ds2 = [(a, b, float(x["distance"].split()[0]), s) for (a, b, _, s), x in zip(ds, inp.get("distances", []))]
            res = solve(c, u, ds2, [], [(s, t, parse_dms(x["direction"]), sd) for (s, t, _, sd), x in zip(di, inp["directions"])])
        exp = {"result.degrees_of_freedom": float(res["r"]), "result.test": res["test"]}
        if res["s02"] is not None:
            exp["result.reference_variance"] = res["s02"]
        for k, p in enumerate(res["points"]):
            for key in ("northing", "easting", "semi_major", "semi_minor", "sd_northing"):
                exp[f"result.adjusted.{k}.{key}.value"] = p[key]
            exp[f"result.adjusted.{k}.orientation.value"] = p["orientation"]
        if res["test"] == "failed":
            exp["meta.warnings.0.code"] = "ADJUSTMENT_TEST_FAILED"
        if res["r"] == 0:
            exp["meta.warnings.1.code"] = "NO_REDUNDANCY"
        exp["ok"] = True
        tol = {}
        for k, v in exp.items():
            if isinstance(v, float):
                tol[k] = ({"abs": 1e-6} if k.endswith(("northing.value", "easting.value")) else {"abs": 0.02} if "orientation" in k
                          else {"rel": 1e-5, "abs": 1e-6} if ("semi" in k or "sd_" in k or "variance" in k) else {"abs": 0})
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": VER, "tolerance": tol})
    # 6. One distance cannot fix a point.
    inp = net_inputs(ctl2, [("P", 1300.0, 1200.0)], [("A", "P", 360.555, 0.005)], [], [])
    out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": "NO_SOLUTION"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "survey.cogo.least-squares-2d.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    for v in out:
        print(v["id"], v["expect"].get("result.test"), v["expect"].get("result.reference_variance"))


if __name__ == "__main__":
    main()
