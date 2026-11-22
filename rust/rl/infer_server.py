"""Batched GPU inference server for Rust `selfplay` (`net=tcp://127.0.0.1:PORT`).

Many Rust game threads (e.g. `--jobs 64`) send observations through a single `BatchClient` in `net.rs` over loopback
TCP; this server runs one bf16/fp32 batched PyTorch forward pass on CUDA (or MPS) and returns `(value, logits, h)`.
At connection start it sends the embedding table and belief linear layer once so `Net::belief` stays local in Rust.

    python infer_server.py --dir runs/loop5/r8 --port 18400 [--compile]
"""

import argparse
import socket
import struct
import sys
import threading
import time

import numpy as np
import torch

from model import ACT_F, BUCKETS, ENT_F, GLOB_F, Tx, load_dir
from train import device as pick_device


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


class InferEngine:
    def __init__(self, net, dev, amp):
        self.net = net
        self.dev = dev
        self.amp = amp and dev.type == "cuda"
        self.lock = threading.Lock()
        self.batches = 0
        self.items = 0

    def forward_batch(self, globs, ent_ids_l, ents_l, act_ids_l, acts_l):
        B = len(globs)
        max_e = max((len(x) for x in ent_ids_l), default=0)
        # Bucket padded widths so CUDA/MPS allocator & torch.compile only see a handful of shapes
        w_e = max(8, ((max_e + 7) // 8) * 8)
        max_a = max((len(x) for x in act_ids_l), default=0)
        w_a = max(4, ((max_a + 3) // 4) * 4)

        glob = np.stack(globs, axis=0)
        ents = np.zeros((B, w_e, ENT_F), dtype=np.float32)
        eids = np.zeros((B, w_e), dtype=np.int64)
        em = np.zeros((B, w_e), dtype=np.bool_)
        acts = np.zeros((B, w_a, ACT_F), dtype=np.float32)
        aids = np.zeros((B, w_a), dtype=np.int64)
        am = np.zeros((B, w_a), dtype=np.bool_)
        n_acts = []
        for i in range(B):
            ne = len(ent_ids_l[i])
            if ne > 0:
                eids[i, :ne] = ent_ids_l[i]
                ents[i, :ne] = ents_l[i]
                em[i, :ne] = True
            na = len(act_ids_l[i])
            n_acts.append(na)
            if na > 0:
                aids[i, :na] = act_ids_l[i]
                acts[i, :na] = acts_l[i]
                am[i, :na] = True

        dev = self.dev
        b = {
            "glob": torch.from_numpy(glob).to(dev),
            "ents": torch.from_numpy(ents).to(dev),
            "ent_ids": torch.from_numpy(eids).to(dev),
            "ent_mask": torch.from_numpy(em).to(dev),
            "acts": torch.from_numpy(acts).to(dev),
            "act_ids": torch.from_numpy(aids).to(dev),
            "act_mask": torch.from_numpy(am).to(dev),
        }
        with self.lock, torch.inference_mode(), torch.autocast("cuda", dtype=torch.bfloat16, enabled=self.amp):
            logits, v, h = self.net(b)
        logits_np = logits.float().cpu().numpy()
        v_np = v.float().cpu().numpy()
        h_np = h.float().cpu().numpy()
        self.batches += 1
        self.items += B
        return logits_np, v_np, h_np, n_acts


def handle_client(conn, engine, handshake):
    try:
        conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        conn.sendall(handshake)
        while True:
            hdr = recv_exact(conn, 8)
            B, payload_bytes = struct.unpack("<II", hdr)
            if B == 0:
                break
            raw = recv_exact(conn, payload_bytes)
            mv = memoryview(raw)
            off = 0
            globs, ent_ids_l, ents_l, act_ids_l, acts_l = [], [], [], [], []
            for _ in range(B):
                n_ent, n_act = struct.unpack_from("<HH", mv, off)
                off += 4
                globs.append(np.frombuffer(mv, dtype=np.float32, count=GLOB_F, offset=off))
                off += GLOB_F * 4
                ent_ids_l.append(np.frombuffer(mv, dtype=np.uint16, count=n_ent, offset=off))
                off += n_ent * 2
                if n_ent > 0:
                    ents_l.append(np.frombuffer(mv, dtype=np.float32, count=n_ent * ENT_F, offset=off).reshape(n_ent, ENT_F))
                    off += n_ent * ENT_F * 4
                else:
                    ents_l.append(None)
                act_ids_l.append(np.frombuffer(mv, dtype=np.uint16, count=n_act, offset=off))
                off += n_act * 2
                if n_act > 0:
                    acts_l.append(np.frombuffer(mv, dtype=np.float32, count=n_act * ACT_F, offset=off).reshape(n_act, ACT_F))
                    off += n_act * ACT_F * 4
                else:
                    acts_l.append(None)

            logits_np, v_np, h_np, n_acts = engine.forward_batch(globs, ent_ids_l, ents_l, act_ids_l, acts_l)
            trunk = h_np.shape[1]
            chunks = [struct.pack("<II", B, trunk)]
            for i in range(B):
                na = n_acts[i]
                chunks.append(struct.pack("<fH", float(v_np[i]), na))
                if na > 0:
                    chunks.append(logits_np[i, :na].astype(np.float32, copy=False).tobytes())
                chunks.append(h_np[i].astype(np.float32, copy=False).tobytes())
            conn.sendall(b"".join(chunks))
    except EOFError:
        pass
    except Exception:
        import traceback
        traceback.print_exc()
        sys.stdout.flush()
    finally:
        conn.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True, help="run folder with net.pt and arch.json")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=18400)
    ap.add_argument("--amp", type=int, default=1)
    ap.add_argument("--compile", action="store_true")
    args = ap.parse_args()

    dev = pick_device()
    net = load_dir(args.dir, dev).eval()
    handshake = pack_handshake(net)
    if args.compile and dev.type == "cuda":
        net = torch.compile(net)
    engine = InferEngine(net, dev, bool(args.amp))

    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((args.host, args.port))
    port = srv.getsockname()[1]
    srv.listen(16)
    print(f"INFER_SERVER ready tcp://{args.host}:{port} device={dev} amp={engine.amp}", flush=True)
    t0 = time.time()
    try:
        while True:
            conn, _ = srv.accept()
            threading.Thread(target=handle_client, args=(conn, engine, handshake), daemon=True).start()
    except KeyboardInterrupt:
        pass
    finally:
        dt = max(time.time() - t0, 1e-6)
        print(f"INFER_SERVER stopped: {engine.items} evals in {engine.batches} batches ({engine.items / dt:.0f} eval/s)", flush=True)
        srv.close()


if __name__ == "__main__":
    main()
