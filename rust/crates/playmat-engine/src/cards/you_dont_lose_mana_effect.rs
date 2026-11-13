//! `YouDontLoseManaEffect`: "You don't lose unspent red mana as steps and phases end." (Leyline Tyrant, Electro,
//! Assaulting Battery; "until end of turn, …" as a spell's: The Last Agni Kai)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::keeps_mana("YouDontLoseManaEffect", keeps).args(&["duration", "text"])];

fn keeps(_: &Game, cx: &StaticCx, seat: Seat) -> [bool; 6] {
    let mut out = [false; 6];
    if seat == cx.controller {
        if let Some(k) = cx.args.get(1).and_then(|a| mana_type(a)) {
            out[k] = true;
        }
    }
    out
}
