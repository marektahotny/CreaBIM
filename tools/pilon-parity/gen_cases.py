#!/usr/bin/env python3
"""Networks for comparing the Rust water calculation with PilonHydraulika's.

Writes crates/creabim-services/tests/parity/<name>.json; the C# harness in
this folder then writes <name>.expected.json beside each. Random, but from
fixed seeds, so the same files come out every time.
"""
import json
import os
import random
import sys

OUT = os.path.join(os.path.dirname(__file__), "..", "..", "crates", "creabim-services", "tests", "parity")

SERIES = {
    "Meď": (1.5e-6, [(12, 10), (15, 13), (18, 16), (22, 20), (28, 25), (35, 32), (42, 39), (54, 50)]),
    "Alpex": (7e-6, [(16, 12), (20, 16), (26, 20), (32, 26), (40, 33), (50, 42)]),
    "PPR": (7e-6, [(20, 13.2), (25, 16.6), (32, 21.2), (40, 26.6), (50, 33.4), (63, 42)]),
    "Oceľ závitová": (1.5e-4, [(15, 16.0), (20, 21.6), (25, 27.2), (32, 35.9), (40, 41.8), (50, 53.0)]),
}
FIXTURES = ["Umývadlo", "Vaňa", "Sprcha", "WC kombi", "Kuchynský drez", "Práčka",
            "Umývačka riadu", "Výlevka", "Pisoár", "Bidet", "Záhradný ventil", "Something"]
ROLES = ["shut_off", "check_valve", "strainer", "balancing", "water_meter", "valve_box", "other"]


class Builder:
    def __init__(self, rnd):
        self.rnd = rnd
        self.nodes = []

    def add(self, parent, **kw):
        n = {"id": str(len(self.nodes) + 1), "parent": parent}
        n.update(kw)
        self.nodes.append(n)
        return len(self.nodes) - 1

    def pipe(self, parent, a, b, material, nominal=None, branch=False):
        rough, sizes = SERIES[material]
        if nominal is None:
            nominal, inner = self.rnd.choice(sizes[:4])
        else:
            inner = next(i for n, i in sizes if n == nominal)
        n = {"kind": "pipe", "p1": a, "p2": b, "nominal": nominal, "inner": inner,
             "roughness": rough, "material": material, "sizes": [list(s) for s in sizes],
             "branch": branch}
        if self.rnd.random() < 0.05:
            m = self.rnd.choice(sizes)
            n["manual"] = list(m)
        if self.rnd.random() < 0.05:
            n["zeta"] = round(self.rnd.uniform(0, 4), 2)
        return self.add(parent, **n)

    def point(self, parent, kind, at, **kw):
        return self.add(parent, kind=kind, p1=at, p2=at, **kw)


def grow(b, parent, at, material, depth, branch=False):
    rnd = b.rnd
    # a run of one to three pipes, with elbows or accessories between
    for k in range(rnd.randint(1, 3)):
        axis = rnd.randrange(3)
        d = [0.0, 0.0, 0.0]
        d[axis] = round(rnd.uniform(0.3, 6.0) * rnd.choice([1, -1]), 2)
        if axis == 2:
            d[2] = abs(d[2]) if rnd.random() < 0.8 else -abs(d[2]) / 3
        to = [round(at[i] + d[i], 3) for i in range(3)]
        parent = b.pipe(parent, at, to, material, branch=branch and k == 0)
        at = to
        r = rnd.random()
        if r < 0.3:
            angle = rnd.choice([90.0, 90.0, 45.0])
            parent = b.point(parent, "fitting", at, fitting="elbow", angle=angle)
        elif r < 0.4:
            parent = b.point(parent, "accessory", at, role=rnd.choice(ROLES))
        elif r < 0.45 and material != "Meď":
            material = rnd.choice(list(SERIES))
            parent = b.point(parent, "fitting", at, fitting="transition")
    if depth <= 0 or rnd.random() < 0.25:
        if rnd.random() < 0.05:
            b.point(parent, "equipment", at, linked=True, leaf_required_pa=round(rnd.uniform(50e3, 150e3)))
        elif rnd.random() < 0.03:
            b.point(parent, "equipment", at)
        else:
            dn = rnd.choice([0, 0, 0, 15, 20]) if rnd.random() < 0.3 else 0
            b.point(parent, "fixture", at, fixture=rnd.choice(FIXTURES), connector_dn=dn)
        return
    kids = rnd.choice([2, 2, 2, 3])
    t = b.point(parent, "fitting", at, fitting="tee" if kids == 2 else "cross")
    for k in range(kids):
        grow(b, t, at, material, depth - 1, branch=k > 0)


def case(seed):
    rnd = random.Random(seed)
    b = Builder(rnd)
    material = rnd.choice(list(SERIES))
    _, sizes = SERIES[material]
    root = b.pipe(None, [0.0, 0.0, -1.0], [round(rnd.uniform(1, 15), 2), 0.0, -1.0], material,
                  nominal=rnd.choice(sizes[2:6])[0])
    at = b.nodes[root]["p2"]
    grow(b, root, at, material, rnd.randint(1, 4))
    voda = {}
    if rnd.random() < 0.4:
        voda["MetodaSucasnosti"] = rnd.choice(["DIN1988_300", "STN736655", "PN_B_01706"])
        voda["TypBudovy"] = rnd.choice(["Bytovy", "Administrativny", "Skola", "Hotel", "Nemocnica", "DomovDochodcov"])
    if rnd.random() < 0.3:
        voda["LenZvacsovat"] = False
    if rnd.random() < 0.2:
        voda["DodrzatPostupnostDimenzii"] = False
    if rnd.random() < 0.2:
        voda["MaxR_Pa_m"] = rnd.choice([150.0, 300.0])
    if rnd.random() < 0.2:
        voda["DimenziaPodlaPripojkyPredmetu"] = False
    if rnd.random() < 0.15:
        voda["VodomerJeMimoModelu"] = False
    if rnd.random() < 0.15:
        voda["AutomatickyZvacsitKritickuVetvu"] = False
    if rnd.random() < 0.15:
        voda["MetodaSucasnosti"] = "TabulkaLU"
        voda["TabulkaLU"] = [[0, 0], [5, 0.4], [20, 0.9], [100, 2.0], [500, 4.5]]
    settings = {"Voda": voda} if voda else {}
    if rnd.random() < 0.2:
        settings["Armatury"] = {"NavrhnutRedukcnyVentil": False}
    kind = "hot" if rnd.random() < 0.3 else "cold"
    available = rnd.choice([None, None, None, 120.0, 200.0, 280.0])
    return {"name": f"random-{seed}", "kind": kind, "root_is_connection": rnd.random() < 0.85,
            "available_kpa": available, "settings": settings, "nodes": b.nodes}


def main():
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 40
    os.makedirs(OUT, exist_ok=True)
    for seed in range(1, count + 1):
        c = case(seed)
        with open(os.path.join(OUT, c["name"] + ".json"), "w", encoding="utf-8") as f:
            json.dump(c, f, ensure_ascii=False, indent=1)
            f.write("\n")


if __name__ == "__main__":
    main()
