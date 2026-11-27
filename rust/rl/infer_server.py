"""Batched GPU inference server for Rust `selfplay`/`archbench` (`net=tcp://127.0.0.1:PORT`).

Zero-Python-loop flat protocol + Cross-Thread Dynamic Micro-Batching:
Rust's `BatchClient` sends per-thread flat batches (`vloss` waves). A dedicated GPU worker thread coalesces
concurrent client batches arriving within `--wait-ms` (up to `--max-batch` items) into a single super-batch,
runs one bf16 (optionally `torch.compile`d) forward pass on CUDA, and scatters results back to waiting threads.

    python infer_server.py --dir runs/phase5-final-1.3m-v4-soup --port 18400 [--max-batch 512 --wait-ms 0.5 --compile]
    python infer_server.py --pmnet path/to/net.pmnet --port 0 [--device auto|cuda|mps|cpu] [--amp 1]
"""

import argparse
import json
import queue
import socket
import struct
import sys
import threading
import time

import numpy as np
import torch

from model import ACT_F, BUCKETS, ENT_F, GLOB_F, NO_ENT, Net, Tx, hl_gauss_to_logit, load_dir
from train import device as pick_device


def load_pmnet(path, dev):
    raw = open(path, "rb").read()
    nl = raw.index(b"\n")
    header = json.loads(raw[:nl])
    off = nl + 1
    sd = {}
    for name, shape in header["tensors"]:
        n = 1
        for d in shape:
            n *= d
        arr = np.frombuffer(raw, dtype="<f4", count=n, offset=off).copy()
        sd[name] = torch.from_numpy(arr).view(*shape)
        off += 4 * n
    net = Tx(**header["config"]) if header.get("arch") == "tx" else Net(belief="belief.weight" in sd)
    # Zero-pad legacy v1/v2 global input weights (65 or 81 cols) to current GLOB_F=89 to match playmat-ai/net.rs
    cur_sd = net.state_dict()
    for k, w in list(sd.items()):
        if k in cur_sd and w.ndim == 2 and w.shape[0] == cur_sd[k].shape[0] and w.shape[1] < cur_sd[k].shape[1]:
            padded = torch.zeros_like(cur_sd[k])
            padded[:, : w.shape[1]] = w
            sd[k] = padded
    net.load_state_dict(sd)
    return net.to(dev).eval()


def recv_exact(conn, n):
    buf = bytearray(n)
    view = memoryview(buf)
    got = 0
    while got < n:
        r = conn.recv_into(view[got:])
        if r == 0:
            raise EOFError("client closed")
        got += r
    return buf


def pack_handshake(net):
    if isinstance(net, Tx):
        emb = net.emb.table().detach().float().cpu().contiguous()
        dim = emb.shape[1]
        trunk = net.trunk1.out_features
        bw = net.belief.weight.detach().float().cpu().contiguous()
        bb = net.belief.bias.detach().float().cpu().contiguous()
        has_bel = 1
    else:
        emb = net.emb.weight.detach().float().cpu().contiguous()
        dim = emb.shape[1]
        trunk = net.trunk2.out_features
        has_bel = 1 if net.belief is not None else 0
        bw = net.belief.weight.detach().float().cpu().contiguous() if has_bel else torch.zeros(0)
        bb = net.belief.bias.detach().float().cpu().contiguous() if has_bel else torch.zeros(0)
    hdr = struct.pack("<IIIB", dim, BUCKETS, trunk, has_bel)
    if has_bel:
        return hdr + emb.numpy().tobytes() + bw.numpy().tobytes() + bb.numpy().tobytes()
    return hdr


def unpack_flat(B, w_e, w_a, mv):
    off = 0
    glob = np.frombuffer(mv, dtype=np.float32, count=B * GLOB_F, offset=off).reshape(B, GLOB_F)
    off += B * GLOB_F * 4
    eids = np.frombuffer(mv, dtype=np.uint16, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e * 2
    ents = np.frombuffer(mv, dtype=np.float32, count=B * w_e * ENT_F, offset=off).reshape(B, w_e, ENT_F)
    off += B * w_e * ENT_F * 4
    em = np.frombuffer(mv, dtype=np.bool_, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    eridx = np.frombuffer(mv, dtype=np.uint8, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    ertyp = np.frombuffer(mv, dtype=np.uint8, count=B * w_e, offset=off).reshape(B, w_e)
    off += B * w_e
    aids = np.frombuffer(mv, dtype=np.uint16, count=B * w_a, offset=off).reshape(B, w_a)
    off += B * w_a * 2
    acts = np.frombuffer(mv, dtype=np.float32, count=B * w_a * ACT_F, offset=off).reshape(B, w_a, ACT_F)
    off += B * w_a * ACT_F * 4
    am = np.frombuffer(mv, dtype=np.bool_, count=B * w_a, offset=off).reshape(B, w_a)
    off += B * w_a
    aent = np.frombuffer(mv, dtype=np.uint8, count=B * w_a, offset=off).reshape(B, w_a)
    return (glob, eids, ents, em, eridx, ertyp, aids, acts, am, aent)


class _Req:
    __slots__ = ("B", "w_e", "w_a", "arrs", "ev", "out", "err")

    def __init__(self, B, w_e, w_a, arrs):
        self.B = B
        self.w_e = w_e
        self.w_a = w_a
        self.arrs = arrs
        self.ev = threading.Event()
        self.out = None
        self.err = None


class InferEngine:
    def __init__(self, net, dev, amp, max_batch=512, wait_ms=0.5):
        self.net = net
        self.dev = dev
        self.amp = amp and dev.type == "cuda"
        self.max_batch = max(1, int(max_batch))
        self.wait_s = max(0.0, float(wait_ms)) / 1000.0
        self.q = queue.Queue()
        self.stop_ev = threading.Event()
        self.reqs = 0
        self.gpu_batches = 0
        self.items = 0
        self.worker = threading.Thread(target=self._loop, daemon=True)
        self.worker.start()

    def stop(self):
        self.stop_ev.set()
        self.q.put(None)
        self.worker.join(timeout=2.0)

    def submit_flat(self, B, w_e, w_a, mv):
        req = _Req(B, w_e, w_a, unpack_flat(B, w_e, w_a, mv))
        self.q.put(req)
        req.ev.wait()
        if req.err is not None:
            raise req.err
        return req.out

    def _run_superbatch(self, group):
        dev = self.dev
        nb = self.dev.type == "cuda"
        if len(group) == 1:
            r = group[0]
            glob, eids, ents, em, eridx, ertyp, aids, acts, am, aent = r.arrs
            tot_B, max_e, max_a = r.B, r.w_e, r.w_a
        else:
            tot_B = sum(r.B for r in group)
            max_e = max(r.w_e for r in group)
            max_a = max(r.w_a for r in group)
            glob = np.empty((tot_B, GLOB_F), dtype=np.float32)
            eids = np.zeros((tot_B, max_e), dtype=np.uint16)
            ents = np.zeros((tot_B, max_e, ENT_F), dtype=np.float32)
            em = np.zeros((tot_B, max_e), dtype=np.bool_)
            eridx = np.full((tot_B, max_e), NO_ENT, dtype=np.uint8)
            ertyp = np.zeros((tot_B, max_e), dtype=np.uint8)
            aids = np.zeros((tot_B, max_a), dtype=np.uint16)
            acts = np.zeros((tot_B, max_a, ACT_F), dtype=np.float32)
            am = np.zeros((tot_B, max_a), dtype=np.bool_)
            aent = np.full((tot_B, max_a), NO_ENT, dtype=np.uint8)
            row = 0
            for r in group:
                g, ei, en, m_e, ri, rt, ai, ac, m_a, ae = r.arrs
                b0, b1 = row, row + r.B
                we, wa = r.w_e, r.w_a
                glob[b0:b1] = g
                eids[b0:b1, :we] = ei
                ents[b0:b1, :we] = en
                em[b0:b1, :we] = m_e
                eridx[b0:b1, :we] = ri
                ertyp[b0:b1, :we] = rt
                aids[b0:b1, :wa] = ai
                acts[b0:b1, :wa] = ac
                am[b0:b1, :wa] = m_a
                aent[b0:b1, :wa] = ae
                row = b1

        b = {
            "glob": torch.from_numpy(glob).to(dev, non_blocking=nb),
            "ents": torch.from_numpy(ents).to(dev, non_blocking=nb),
            "ent_ids": torch.from_numpy(eids.astype(np.int64)).to(dev, non_blocking=nb),
            "ent_mask": torch.from_numpy(em).to(dev, non_blocking=nb),
            "ent_rel_idx": torch.from_numpy(eridx.astype(np.int64)).to(dev, non_blocking=nb),
            "ent_rel_type": torch.from_numpy(ertyp.astype(np.int64)).to(dev, non_blocking=nb),
            "acts": torch.from_numpy(acts).to(dev, non_blocking=nb),
            "act_ids": torch.from_numpy(aids.astype(np.int64)).to(dev, non_blocking=nb),
            "act_mask": torch.from_numpy(am).to(dev, non_blocking=nb),
            "act_ent_idx": torch.from_numpy(aent.astype(np.int64)).to(dev, non_blocking=nb),
        }
        with torch.inference_mode(), torch.autocast("cuda", dtype=torch.bfloat16, enabled=self.amp):
            logits, v, h = self.net(b)
        v_np = hl_gauss_to_logit(v).cpu().contiguous().numpy()
        logits_np = logits.float().cpu().contiguous().numpy()
        h_np = h.float().cpu().contiguous().numpy()
        self.gpu_batches += 1
        self.reqs += len(group)
        self.items += tot_B

        if len(group) == 1:
            group[0].out = (v_np, logits_np, h_np)
            group[0].ev.set()
        else:
            row = 0
            for r in group:
                b0, b1 = row, row + r.B
                v_i = np.ascontiguousarray(v_np[b0:b1])
                l_i = np.ascontiguousarray(logits_np[b0:b1, : r.w_a])
                h_i = np.ascontiguousarray(h_np[b0:b1])
                r.out = (v_i, l_i, h_i)
                r.ev.set()
                row = b1

    def _loop(self):
        while not self.stop_ev.is_set():
            first = self.q.get()
            if first is None:
                break
            group = [first]
            tot_B = first.B
            if self.wait_s > 0.0 and tot_B < self.max_batch:
                deadline = time.perf_counter() + self.wait_s
                while tot_B < self.max_batch:
                    rem = deadline - time.perf_counter()
                    if rem <= 0.0:
                        break
                    try:
                        nxt = self.q.get(timeout=rem)
                    except queue.Empty:
                        break
                    if nxt is None:
                        self.stop_ev.set()
                        break
                    group.append(nxt)
                    tot_B += nxt.B
            else:
                while tot_B < self.max_batch:
                    try:
                        nxt = self.q.get_nowait()
                    except queue.Empty:
                        break
                    if nxt is None:
                        self.stop_ev.set()
                        break
                    group.append(nxt)
                    tot_B += nxt.B
            try:
                self._run_superbatch(group)
            except Exception as e:
                for r in group:
                    r.err = e
                    r.ev.set()


def handle_client(conn, engine, handshake):
    try:
        conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        conn.sendall(handshake)
        while True:
            hdr = recv_exact(conn, 12)
            B, w_e, w_a = struct.unpack("<III", hdr)
            if B == 0:
                break
            payload_bytes = (
                B * GLOB_F * 4
                + B * w_e * 2
                + B * w_e * ENT_F * 4
                + B * w_e * 3
                + B * w_a * 2
                + B * w_a * ACT_F * 4
                + B * w_a * 2
            )
            raw = recv_exact(conn, payload_bytes)
            v_np, logits_np, h_np = engine.submit_flat(B, w_e, w_a, memoryview(raw))
            trunk = h_np.shape[1]
            conn.sendall(struct.pack("<II", B, trunk) + v_np.tobytes() + logits_np.tobytes() + h_np.tobytes())
    except (EOFError, ConnectionResetError, BrokenPipeError):
        pass
    except Exception:
        import traceback
        traceback.print_exc()
        sys.stdout.flush()
    finally:
        if engine.gpu_batches > 0:
            print(
                f"INFER_SERVER client done: {engine.items} evals in {engine.gpu_batches} gpu_batches "
                f"({engine.reqs} reqs, avg_gpu_B={engine.items / engine.gpu_batches:.1f}, "
                f"coalesce={engine.reqs / engine.gpu_batches:.2f}x)",
                flush=True,
            )
        conn.close()


def main():
    ap = argparse.ArgumentParser()
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--dir", help="run folder with net.pt and arch.json")
    src.add_argument("--pmnet", help="exported .pmnet file to serve directly")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=18400)
    ap.add_argument("--device", default="auto", choices=["auto", "cuda", "mps", "cpu"])
    ap.add_argument("--amp", type=int, default=1)
    ap.add_argument("--max-batch", type=int, default=512, help="max super-batch items across concurrent threads")
    ap.add_argument("--wait-ms", type=float, default=0.5, help="micro-batch coalescing window in ms")
    ap.add_argument("--compile", action="store_true")
    args = ap.parse_args()

    dev = pick_device() if args.device == "auto" else torch.device(args.device)
    if dev.type == "cuda":
        torch.set_float32_matmul_precision("high")
    net = load_pmnet(args.pmnet, dev) if args.pmnet else load_dir(args.dir, dev).eval()
    handshake = pack_handshake(net)
    if args.compile and dev.type == "cuda":
        net = torch.compile(net, dynamic=True)
    engine = InferEngine(net, dev, bool(args.amp), max_batch=args.max_batch, wait_ms=args.wait_ms)

    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((args.host, args.port))
    port = srv.getsockname()[1]
    srv.listen(128)
    print(
        f"INFER_SERVER ready tcp://{args.host}:{port} device={dev} amp={engine.amp} "
        f"max_batch={args.max_batch} wait_ms={args.wait_ms} compile={args.compile}",
        flush=True,
    )
    t0 = time.time()
    try:
        while True:
            conn, _ = srv.accept()
            threading.Thread(target=handle_client, args=(conn, engine, handshake), daemon=True).start()
    except KeyboardInterrupt:
        pass
    finally:
        engine.stop()
        dt = max(time.time() - t0, 1e-6)
        gb = max(engine.gpu_batches, 1)
        print(
            f"INFER_SERVER stopped: {engine.items} evals in {engine.gpu_batches} gpu_batches "
            f"(avg_gpu_B={engine.items / gb:.1f}, {engine.items / dt:.0f} eval/s)",
            flush=True,
        )
        srv.close()


if __name__ == "__main__":
    main()
