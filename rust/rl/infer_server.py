"""Batched GPU inference server for Rust `selfplay` (`net=tcp://127.0.0.1:PORT`).

Zero-Python-loop flat protocol: Rust's `BatchClient` pre-pads and packs each batch into 7 contiguous arrays, so this
server slices directly with zero-copy `np.frombuffer`, runs one bf16 batched PyTorch forward pass on CUDA (or MPS),
and sends back flat `(v, logits, h)` arrays in one write.

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

    def forward_flat(self, B, w_e, w_a, mv):
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

        dev = self.dev
        b = {
            "glob": torch.from_numpy(glob).to(dev),
            "ents": torch.from_numpy(ents).to(dev),
            "ent_ids": torch.from_numpy(eids.astype(np.int64)).to(dev),
            "ent_mask": torch.from_numpy(em).to(dev),
            "ent_rel_idx": torch.from_numpy(eridx.astype(np.int64)).to(dev),
            "ent_rel_type": torch.from_numpy(ertyp.astype(np.int64)).to(dev),
            "acts": torch.from_numpy(acts).to(dev),
            "act_ids": torch.from_numpy(aids.astype(np.int64)).to(dev),
            "act_mask": torch.from_numpy(am).to(dev),
            "act_ent_idx": torch.from_numpy(aent.astype(np.int64)).to(dev),
        }
        with self.lock, torch.inference_mode(), torch.autocast("cuda", dtype=torch.bfloat16, enabled=self.amp):
            logits, v, h = self.net(b)
        v_np = v.float().cpu().contiguous().numpy()
        logits_np = logits.float().cpu().contiguous().numpy()
        h_np = h.float().cpu().contiguous().numpy()
        self.batches += 1
        self.items += B
        return v_np, logits_np, h_np


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
            v_np, logits_np, h_np = engine.forward_flat(B, w_e, w_a, memoryview(raw))
            trunk = h_np.shape[1]
            conn.sendall(struct.pack("<II", B, trunk) + v_np.tobytes() + logits_np.tobytes() + h_np.tobytes())
    except (EOFError, ConnectionResetError, BrokenPipeError):
        pass
    except Exception:
        import traceback
        traceback.print_exc()
        sys.stdout.flush()
    finally:
        if engine.batches > 0:
            print(f"INFER_SERVER client done: {engine.items} evals in {engine.batches} batches (avg B={engine.items / engine.batches:.1f})", flush=True)
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
