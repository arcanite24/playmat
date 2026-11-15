//! Shantotto, Tactician Magician: "Whenever you cast a noncreature spell, Shantotto gets +X/+0 until end of turn,
//! where X is the amount of mana spent to cast that spell. If X is 4 or more, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("ShantottoTacticianMagician.ShantottoTacticianMagicianValue", spent),
    Handler::condition("ShantottoTacticianMagician.ShantottoTacticianMagicianCondition", four_or_more),
];

/// the trigger's spell (`getValue("spellCast")`): the mana spent to cast it (`ManaPaidSourceWatcher.getTotalPaid`)
fn mana_spent(game: &Game) -> i32 {
    game.resolving_item.as_ref().and_then(|r| r.cast_spell).map_or(0, |s| game.obj(s).mana_spent as i32)
}

fn spent(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    mana_spent(game)
}

fn four_or_more(game: &Game, _you: Seat, _source: ObjId) -> bool {
    mana_spent(game) >= 4
}
