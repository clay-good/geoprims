#!/usr/bin/env python3
"""Golden vectors for survey.reduction.level-network, worked with NumPy's
least squares (lstsq on the weighted design matrix), independent of the
core's Cholesky. Prints JSON lines; writes nothing."""
import json
import random
import numpy as np

SRC = "Weighted least squares worked with NumPy lstsq (tools/vectors/gen_level_network.py)"


def adjust(fixed, obs):
    names = []
    for o in obs:
        for n in (o["from"], o["to"]):
            if n not in fixed and n not in names:
                names.append(n)
    A = np.zeros((len(obs), len(names)))
    l = np.zeros(len(obs))
    w = np.zeros(len(obs))
    for k, o in enumerate(obs):
        if o["to"] in names:
            A[k, names.index(o["to"])] = 1
        if o["from"] in names:
            A[k, names.index(o["from"])] = -1
        l[k] = o["rise"] - fixed.get(o["to"], 0.0) + fixed.get(o["from"], 0.0)
        w[k] = o["w"]
    sw = np.sqrt(w)
    x, *_ = np.linalg.lstsq(A * sw[:, None], l * sw, rcond=None)
    v = A @ x - l
    dof = len(obs) - len(names)
    s02 = float((w * v * v).sum() / dof) if dof > 0 else None
    return names, x, s02, dof


def fmt_m(x):
    return f"{x:.4f} m"


def vectors():
    out = []
    i = 0
    rnd = random.Random(42)
    # Networks: a true elevation set, runs with small errors, weights by length.
    for case in range(18):
        true = {"BM A": 100.0}
        n_bm = 3 + case % 6
        for j in range(n_bm):
            true[f"BM {chr(66 + j)}"] = 100.0 + rnd.uniform(-20, 20)
        names = list(true)
        pairs = [(names[j], names[j + 1]) for j in range(len(names) - 1)] + [(names[-1], names[0])]
        extra = [(names[rnd.randrange(len(names))], names[rnd.randrange(len(names))]) for _ in range(case % 4)]
        pairs += [p for p in extra if p[0] != p[1]]
        mode = ("length", "setups", "sd")[case % 3]
        obs = []
        for f, t in pairs:
            rise = true[t] - true[f] + rnd.uniform(-0.004, 0.004)
            o = {"from": f, "to": t, "rise": round(rise, 4)}
            if mode == "length":
                L = round(rnd.uniform(0.5, 3.0), 2)
                o["length"], o["w"] = L, 1 / L
            elif mode == "setups":
                n = rnd.randint(4, 30)
                o["setups"], o["w"] = n, 1 / n
            else:
                sd = round(rnd.uniform(0.002, 0.006), 4)
                o["sd"], o["w"] = sd, 1 / sd ** 2
            obs.append(o)
        fixed = {"BM A": 100.0}
        if case % 5 == 4:
            fixed["BM B"] = round(true["BM B"], 4)
        names_u, x, s02, dof = adjust(fixed, obs)
        i += 1
        inp = {"fixed": [{"name": k, "elevation": fmt_m(h)} for k, h in fixed.items()],
               "observations": [{"from": o["from"], "to": o["to"], "rise": fmt_m(o["rise"]),
                                 **({"length": f"{o['length']} km"} if "length" in o else {}),
                                 **({"setups": o["setups"]} if "setups" in o else {}),
                                 **({"sd": f"{o['sd'] * 1000:.1f} mm"} if "sd" in o else {})} for o in obs]}
        exp = {"ok": True, "result.degrees_of_freedom": float(dof)}
        for k, n in enumerate(names_u):
            exp[f"result.adjusted.{k}.name"] = n
            # As the site displays it: four decimals, trailing zeros dropped.
            shown = f"{x[k]:.4f}".rstrip("0").rstrip(".")
            exp[f"result.adjusted.{k}.elevation"] = f"{shown} m"
        tol = {"result.degrees_of_freedom": {"abs": 0}}
        if s02 is not None:
            exp["result.reference_variance"] = s02
            tol["result.reference_variance"] = {"rel": 1e-6, "abs": 1e-12}
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": "NumPy 2", "tolerance": tol})
    # A benchmark tied to nothing fixed.
    i += 1
    out.append({"id": f"v{i:03d}", "input": {"fixed": [{"name": "BM A", "elevation": "100 m"}], "observations": [{"from": "BM A", "to": "BM B", "rise": "1 m"}, {"from": "BM C", "to": "BM D", "rise": "2 m"}]},
                "expect": {"ok": False, "error.code": "NO_SOLUTION", "error.field": "/observations"}, "source": SRC, "sourceVersion": "NumPy 2", "tolerance": {}})
    # Every benchmark fixed.
    i += 1
    out.append({"id": f"v{i:03d}", "input": {"fixed": [{"name": "BM A", "elevation": "100 m"}, {"name": "BM B", "elevation": "101 m"}], "observations": [{"from": "BM A", "to": "BM B", "rise": "1 m"}]},
                "expect": {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/observations"}, "source": SRC, "sourceVersion": "NumPy 2", "tolerance": {}})
    return out


if __name__ == "__main__":
    for v in vectors():
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
