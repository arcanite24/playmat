//! Awakened Awareness: "Enchant artifact or creature. When Awakened Awareness enters, put X +1/+1 counters on
//! enchanted permanent. As long as enchanted permanent is a creature, it has base power and toughness 1/1."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("AwakenedAwareness.AwakenedAwarenessEffect", enchanted_creature, Layer::SetPt(one_one))];

/// the permanent the source is attached to, while it's a creature
fn enchanted_creature(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id) && game.has_type(id, CardType::Creature)
}

/// `setModifiedBaseValue(1)` for both (layer 7b)
fn one_one(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (1, 1)
}
