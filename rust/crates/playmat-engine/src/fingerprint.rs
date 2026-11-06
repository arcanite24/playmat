//! State fingerprints, text version 2 (docs/rust/StateFingerprint.md): the game as plain text two engines agree on,
//! and its FNV-1a 64 hash. Java: `mage.game.trace.StateFingerprint`.

use crate::state::{Game, ObjId};

pub const VERSION: u32 = 2;

pub fn hash_text(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

pub fn hash(game: &Game) -> String {
    hash_text(&text(game))
}

fn seat(s: Option<usize>) -> String {
    s.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string())
}

pub(crate) fn counters(counters: &[(String, i32)]) -> String {
    let mut sorted: Vec<&(String, i32)> = counters.iter().filter(|(_, n)| *n != 0).collect();
    if sorted.is_empty() {
        return "-".to_string();
    }
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    sorted.iter().map(|(name, n)| format!("{}={}", name, n)).collect::<Vec<_>>().join(",")
}

pub fn text(game: &Game) -> String {
    let mut out = String::new();
    out.push_str(&format!("v{}\n", VERSION));
    out.push_str(&format!(
        "turn {} step {} active {} priority {}\n",
        game.turn,
        game.step.map(|s| s.name()).unwrap_or("-"),
        seat(game.active),
        seat(game.priority)
    ));
    for (i, player) in game.players.iter().enumerate() {
        out.push_str(&format!(
            "player {} life {} counters {}{}{}\n",
            i,
            player.life,
            counters(&player.counters),
            if player.lost { " lost" } else { "" },
            if player.won { " won" } else { "" }
        ));
        // as Java's trace sees the end: the loser's cards are gone from hand, library and graveyard (their
        // permanents stay), though continuous effects still counted them
        let gone = |ids: &[ObjId]| -> Vec<ObjId> { if game.over && player.lost { Vec::new() } else { ids.to_vec() } };
        let (h, l, g) = (gone(&player.hand), gone(&player.library), gone(&player.graveyard));
        let mut hand: Vec<&str> = h.iter().map(|id| game.obj(*id).name()).collect();
        hand.sort();
        out.push_str(&format!(" hand {}\n", hand.join("|")));
        let library: Vec<&str> = l.iter().map(|id| game.obj(*id).name()).collect();
        out.push_str(&format!(" library {}\n", library.join("|")));
        let graveyard: Vec<&str> = g.iter().map(|id| game.obj(*id).name()).collect();
        out.push_str(&format!(" graveyard {}\n", graveyard.join("|")));
    }
    let mut exile: Vec<&str> = game.exile.iter().map(|id| game.obj(*id).name()).collect();
    exile.sort();
    out.push_str(&format!("exile {}\n", exile.join("|")));
    let mut battlefield: Vec<String> = game
        .battlefield
        .iter()
        .map(|id| {
            let o = game.obj(*id);
            let (p, t) = game.power_toughness(*id);
            format!(
                "{} {}{}{} {}/{} dmg {}{}{} counters {}",
                seat(Some(o.controller)),
                o.name(),
                if o.tapped { " T" } else { " U" },
                if o.face_down { " facedown" } else { "" },
                p,
                t,
                o.damage,
                if o.attacking.is_some() { " attacking" } else { "" },
                if o.blocks > 0 { " blocking" } else { "" },
                counters(&o.counters)
            )
        })
        .collect();
    battlefield.sort();
    for line in battlefield {
        out.push_str(&format!("bf {}\n", line));
    }
    for item in game.stack.iter().rev() {
        out.push_str(&format!("stack {} {}\n", item.controller, game.stack_name(item)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_test_vectors() {
        assert_eq!(hash_text(""), "cbf29ce484222325");
        assert_eq!(hash_text("a"), "af63dc4c8601ec8c");
        assert_eq!(hash_text("foobar"), "85944171f73967e8");
    }
}
