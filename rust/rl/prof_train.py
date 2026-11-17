"""Where a training step's time goes, and a check that the torch batches match the NumPy ones.

    python prof_train.py --data data/loop3-r4 --init runs/loop3/r4 [--bs 512] [--steps 60]
"""

import argparse
import time

import numpy as np
import torch

from data import Data
from model import Critic, load_dir, params
from train import device, losses


def timed(f, steps, dev):
    f()
    if dev.type == "cuda":
        torch.cuda.synchronize()
    t = time.time()
    for _ in range(steps):
        f()
    if dev.type == "cuda":
        torch.cuda.synchronize()
    return 1000 * (time.time() - t) / steps


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", action="append", required=True)
    ap.add_argument("--init", required=True)
    ap.add_argument("--bs", type=int, default=512)
    ap.add_argument("--steps", type=int, default=60)
    args = ap.parse_args()
    dev = device()
    data = Data(args.data)
    net = load_dir(args.init, dev)
    critic = Critic().to(dev)
    print(f"PROF {len(data)} decisions, net {params(net)} params, bs {args.bs}")
    rng = np.random.default_rng(0)
    idx = rng.integers(0, len(data), args.bs)

    # the torch batch, padded back to the fixed widths, is the NumPy one
    data.place("cpu")
    a, b = data.batch_np(idx, "cpu"), data.batch(idx, "cpu", trim=False)
    worst = max(float((a[k].float() - b[k].float()).abs().max()) for k in a)
    print(f"PROF torch batch vs numpy batch: largest difference {worst}, same keys {sorted(a) == sorted(b)}")
    with torch.no_grad():
        la = [float(x) for x in losses(net, critic, data.batch_np(idx, dev))]
        lb = [float(x) for x in losses(net, critic, data.batch(idx, dev, trim=True))]
    print(f"PROF losses fixed widths {np.round(la, 5).tolist()}, trimmed {np.round(lb, 5).tolist()}")

    print(f"PROF batch, numpy on the cpu + copy: {timed(lambda: data.batch_np(idx, dev), args.steps, dev):.1f} ms")
    print(f"PROF batch, torch on the cpu + copy: {timed(lambda: data.batch(idx, dev), args.steps, dev):.1f} ms")
    where, size = data.place(dev)
    print(f"PROF data {size:.1f} GB on {where}")
    print(f"PROF batch, torch on {where}: {timed(lambda: data.batch(idx, dev), args.steps, dev):.1f} ms")

    opt = torch.optim.AdamW(list(net.parameters()) + list(critic.parameters()), lr=1e-5, fused=dev.type == "cuda")
    for trim in (False, True):
        bt = data.batch(idx, dev, trim=trim)
        for amp in (False, True):
            def step():
                with torch.autocast("cuda", dtype=torch.bfloat16, enabled=amp):
                    pol, val, bel, crit = losses(net, critic, bt)[:4]
                    loss = pol + 0.5 * val + 0.2 * bel + 0.5 * crit
                opt.zero_grad()
                loss.backward()
                opt.step()
            print(f"PROF gpu step, trim={trim} bf16={amp}: {timed(step, args.steps, dev):.1f} ms"
                  f" (ents width {bt['ents'].shape[1]})")


if __name__ == "__main__":
    main()
