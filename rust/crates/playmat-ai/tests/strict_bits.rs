//! The strict build's floats, to the bit (Mana@Home, docs/swarm/S0.md): every network the tests ship, on the parity
//! fixtures' observations, and `exp`, `ln` and `tanh` on a fixed grid. The engine and the search's own arithmetic are
//! integers and IEEE basics, the same on every machine, so these are what could make a strict worker play another
//! game: CI checks them on x86_64 Linux against hashes recorded on an ARM Mac.
//!
//! A change to the strict kernel, `fm.rs` or a shipped network moves a hash on purpose: print the new ones with
//! `PLAYMAT_STRICT_BITS=print cargo test -p playmat-ai --features strict --test strict_bits -- --nocapture`, check
//! them on a second architecture, and update `WANT`.
#![cfg(feature = "strict")]

use playmat_ai::fm::Fm;
use playmat_ai::net::{self, Net};
use playmat_ai::obs::{self, Obs};
use sha2::{Digest, Sha256};

/// recorded on an M-series Mac (aarch64-apple-darwin), debug and release alike
const WANT: [(&str, &str); 5] = [
    ("builtin", "68359a7d653e5314b061d5604eb7482dcd31aff652433dbb28e52b9f115046c9"),
    ("tx_small", "09c9a2c1cad8ab85870f7d78f4c7cf48a306bbb6d3ecd150f7817b2bc8173f0d"),
    ("tx_v2_small", "38d444ca543eb77ce4109d192ab33c55211afa24689572b98bc77c67a1da0d53"),
    ("tx_rel_small", "290aa0a32c87baaf6d8f12bf4351313c08ce7499bc988614732c1dc936e9de72"),
    ("fm", "4eaa4fcca72c77273c37f58abbe14130000c7f5b110db9f9debf083065e406e0"),
];

#[test]
fn strict_floats_are_the_recorded_bits() {
    let mut got = Vec::new();
    got.push(("builtin", net_bits(&net::load("builtin").unwrap(), "net_parity.json")));
    for (name, fixture) in [
        ("tx_small", "net_parity_tx.json"),
        ("tx_v2_small", "net_parity_tx_v2.json"),
        ("tx_rel_small", "net_parity_tx_rel.json"),
    ] {
        let bytes = std::fs::read(format!("{}/tests/{name}.pmnet", env!("CARGO_MANIFEST_DIR"))).unwrap();
        got.push((name, net_bits(&Net::parse(name, &bytes).unwrap(), fixture)));
    }
    got.push(("fm", fm_bits()));
    if std::env::var("PLAYMAT_STRICT_BITS").as_deref() == Ok("print") {
        for (name, h) in &got {
            println!("    (\"{name}\", \"{h}\"),");
        }
        return;
    }
    for ((name, h), (want_name, want)) in got.iter().zip(WANT) {
        assert_eq!(*name, want_name);
        assert_eq!(h, want, "{name}: the strict build computes other bits than the recorded ones (see the header)");
    }
}

/// sha256 of every logit, value and belief bit the network gives on the fixture's observations
fn net_bits(net: &Net, fixture: &str) -> String {
    let text = std::fs::read_to_string(format!("{}/tests/{fixture}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let cases: serde_json::Value = serde_json::from_str(&text).unwrap();
    let f = |v: &serde_json::Value| -> Vec<f32> {
        v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect()
    };
    let u = |v: &serde_json::Value| -> Vec<u32> {
        v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u32).collect()
    };
    let u8v = |v: &serde_json::Value, n: usize, def: u8| -> Vec<u8> {
        v.as_array().map(|arr| arr.iter().map(|x| x.as_u64().unwrap() as u8).collect()).unwrap_or_else(|| vec![def; n])
    };
    let mut h = Sha256::new();
    for c in cases["cases"].as_array().unwrap() {
        let ent_ids = u(&c["ent_ids"]);
        let act_ids = u(&c["act_ids"]);
        let o = Obs {
            glob: f(&c["glob"]),
            ents: f(&c["ents"]),
            ent_rel_idx: u8v(&c["ent_rel_idx"], ent_ids.len(), obs::NO_ENT),
            ent_rel_type: u8v(&c["ent_rel_type"], ent_ids.len(), 0),
            ent_ids,
            acts: f(&c["acts"]),
            act_ent_idx: u8v(&c["act_ent_idx"], act_ids.len(), obs::NO_ENT),
            act_ids,
        };
        let out = net.forward(&o);
        let bel = net.belief(&out.h, &u(&c["pool"]));
        for x in out.logits.iter().chain(std::iter::once(&out.value)).chain(&bel) {
            h.update(x.to_bits().to_le_bytes());
        }
    }
    hex(&h.finalize())
}

/// sha256 of `exp`, `ln` and `tanh` in f32 and f64 over a grid that covers the ranges the search and the networks use
fn fm_bits() -> String {
    let mut h = Sha256::new();
    for i in -400..=400 {
        let x = i as f64 * 0.0625 + 0.001 * i as f64 * i as f64 / 400.0;
        let p = x.abs() + 1e-6;
        for y in [x.sexp(), p.sln(), x.stanh()] {
            h.update(y.to_bits().to_le_bytes());
        }
        let (x, p) = (x as f32, p as f32);
        for y in [x.sexp(), p.sln(), x.stanh()] {
            h.update(y.to_bits().to_le_bytes());
        }
    }
    hex(&h.finalize())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
