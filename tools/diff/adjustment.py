#!/usr/bin/env python3
"""An independent reference for three survey adjustments, in numpy
(platform/verification, cross-implementation differential testing).

Written from the normal equations, not from the Rust: this solves the same
models with numpy's own dense linear algebra, so a mistake in the core's
Cholesky, in its weighting, or in its covariance propagation shows up as a
disagreement rather than being reproduced.

Reads one JSON object on stdin:

  {"levelNetwork": [{"fixed": {...}, "observations": [...], "weighting": "..."},
                    ...],
   "helmert": [{"source": [[x,y,z],...], "target": [[x,y,z],...],
                "convention": "position-vector" | "coordinate-frame"}, ...],
   "partition": [{"ring": [[e,n],...], "corners": [i,...],
                  "start": [e,n], "end": [e,n]}, ...]}

Writes one JSON object: for each case, the quantities the tool also reports.
"""
import json
import sys

import numpy as np


def level_network(case):
    """Weighted least squares on a levelling net: H_to - H_from = rise + v."""
    fixed = case["fixed"]
    obs = case["observations"]
    unknowns = []
    for o in obs:
        for n in (o["from"], o["to"]):
            if n not in fixed and n not in unknowns:
                unknowns.append(n)
    index = {n: i for i, n in enumerate(unknowns)}
    a = np.zeros((len(obs), len(unknowns)))
    l = np.zeros(len(obs))
    w = np.zeros(len(obs))
    for k, o in enumerate(obs):
        # The observation is the rise from `from` to `to`, so the equation is
        # H_to - H_from - rise = v, with the fixed marks moved to the right.
        const = 0.0
        if o["to"] in index:
            a[k, index[o["to"]]] = 1.0
        else:
            const -= fixed[o["to"]]
        if o["from"] in index:
            a[k, index[o["from"]]] = -1.0
        else:
            const += fixed[o["from"]]
        l[k] = o["rise"] + const
        if case["weighting"] == "sigma":
            w[k] = 1.0 / (o["sigma"] ** 2)
        elif case["weighting"] == "setups":
            w[k] = 1.0 / o["setups"]
        else:
            w[k] = 1.0 / o["length"]
    p = np.diag(w)
    n = a.T @ p @ a
    x = np.linalg.solve(n, a.T @ p @ l)
    v = a @ x - l
    dof = len(obs) - len(unknowns)
    vpv = float(v.T @ p @ v)
    var = vpv / dof if dof > 0 else 0.0
    qxx = np.linalg.inv(n)
    return {
        "adjusted": {n_: float(x[i]) for n_, i in index.items()},
        "residuals": [float(r) for r in v],
        "referenceVariance": var,
        "degreesOfFreedom": dof,
        "vpv": vpv,
        "standardErrors": {n_: float(np.sqrt(var * qxx[i, i])) for n_, i in index.items()},
    }


def helmert(case):
    """Gauss-Newton on the linearized seven-parameter model (IOGP GN 7-2)."""
    src = np.asarray(case["source"], dtype=float)
    dst = np.asarray(case["target"], dtype=float)
    n = len(src)
    # Centroid-reduced, which is what keeps the normal matrix conditioned at
    # earth-centred magnitudes.
    cs = src.mean(axis=0)
    cd = dst.mean(axis=0)
    s = src - cs
    d = dst - cd
    # Sign of the rotation block: the position-vector convention rotates the
    # point, the coordinate-frame convention rotates the axes.
    sign = 1.0 if case["convention"] == "position-vector" else -1.0
    par = np.zeros(7)  # tx, ty, tz, rx, ry, rz, ds
    for _ in range(12):
        tx, ty, tz, rx, ry, rz, ds = par
        omega = sign * np.array([[0.0, -rz, ry], [rz, 0.0, -rx], [-ry, rx, 0.0]])
        model = np.array([tx, ty, tz]) + (1.0 + ds) * (s + s @ omega.T)
        r = (d - model).reshape(-1)
        a = np.zeros((3 * n, 7))
        for k in range(n):
            x, y, z = s[k]
            a[3 * k + 0, 0] = 1.0
            a[3 * k + 1, 1] = 1.0
            a[3 * k + 2, 2] = 1.0
            # d(omega . v)/d(rx, ry, rz), scaled by (1 + ds).
            g = (1.0 + ds) * sign
            a[3 * k + 0, 4] = g * z
            a[3 * k + 0, 5] = -g * y
            a[3 * k + 1, 3] = -g * z
            a[3 * k + 1, 5] = g * x
            a[3 * k + 2, 3] = g * y
            a[3 * k + 2, 4] = -g * x
            a[3 * k + 0, 6] = x + (ry * z - rz * y) * sign
            a[3 * k + 1, 6] = y + (rz * x - rx * z) * sign
            a[3 * k + 2, 6] = z + (rx * y - ry * x) * sign
        step = np.linalg.lstsq(a, r, rcond=None)[0]
        par = par + step
        if np.max(np.abs(step)) < 1e-15:
            break
    tx, ty, tz, rx, ry, rz, ds = par
    omega = sign * np.array([[0.0, -rz, ry], [rz, 0.0, -rx], [-ry, rx, 0.0]])
    # Back to uncentered translations: the fit was on reduced coordinates.
    t_full = cd - (1.0 + ds) * (cs + omega @ cs) + np.array([tx, ty, tz])
    model = np.array([tx, ty, tz]) + (1.0 + ds) * (s + s @ omega.T)
    v = (d - model).reshape(-1)
    dof = 3 * n - 7
    var = float(v @ v) / dof if dof > 0 else 0.0
    return {
        "tx": float(t_full[0]),
        "ty": float(t_full[1]),
        "tz": float(t_full[2]),
        # Radians out; the tool reports seconds of arc.
        "rx": float(rx),
        "ry": float(ry),
        "rz": float(rz),
        "scale": float(ds),
        "referenceVariance": var,
        "degreesOfFreedom": dof,
        "rms": float(np.sqrt(float(v @ v) / (3 * n))),
    }


def partition(case):
    """The area of the cut piece, by the shoelace formula.

    This does not solve the partition. It takes the cut line the tool returned
    and the corners it says are on the cut side, closes that polygon, and
    measures it. So it checks the property the tool claims rather than
    reproducing how it found the line: if the Brent solve stopped early or the
    piece was assembled from the wrong corners, the area comes out wrong here.
    """
    ring = case["ring"]
    pts = [case["start"]] + [ring[i] for i in case["corners"]] + [case["end"]]
    a = 0.0
    for k in range(len(pts)):
        x1, y1 = pts[k]
        x2, y2 = pts[(k + 1) % len(pts)]
        a += x1 * y2 - x2 * y1
    return {"cutArea": abs(a) / 2.0, "corners": len(pts)}


def main():
    cases = json.load(sys.stdin)
    out = {
        "levelNetwork": [level_network(c) for c in cases.get("levelNetwork", [])],
        "helmert": [helmert(c) for c in cases.get("helmert", [])],
        "partition": [partition(c) for c in cases.get("partition", [])],
    }
    json.dump(out, sys.stdout)


if __name__ == "__main__":
    main()
