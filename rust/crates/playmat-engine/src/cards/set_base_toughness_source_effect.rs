//! `SetBaseToughnessSourceEffect(DevotionCount.X)`: "{this}'s toughness is equal to your devotion to <color>"
//! (Daxos, Blessed by the Sun; Tymaret, Chosen from Death)

use super::*;
use crate::card::{Color, Value};

pub const HANDLERS: &[Handler] = &[Handler::continuous("SetBaseToughnessSourceEffect", own, Layer::SetToughness(devotion))
    .args(&["one_of:DevotionCount.W|DevotionCount.U|DevotionCount.B|DevotionCount.R|DevotionCount.G"])];

fn own(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// the controller's devotion to the color (`SubLayer.CharacteristicDefining_7a`)
fn devotion(game: &Game, cx: &StaticCx, _: ObjId) -> i32 {
    let c = match cx.args.first().map(|a| a.trim()) {
        Some("DevotionCount.W") => Color::W,
        Some("DevotionCount.U") => Color::U,
        Some("DevotionCount.B") => Color::B,
        Some("DevotionCount.R") => Color::R,
        _ => Color::G,
    };
    game.value(&Amount::Of(Value::Devotion { c, c2: None }), cx.controller, cx.source, 0)
}
