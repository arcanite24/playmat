//! Tellah, Great Sage: "Whenever you cast a noncreature spell, create a 1/1 colorless Hero creature token. If four or
//! more mana was spent to cast that spell, draw two cards. If eight or more mana was spent to cast that spell,
//! sacrifice Tellah and it deals that much damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("TellahGreatSage.TellahGreatSageCondition.FOUR", four),
    Handler::condition("TellahGreatSage.TellahGreatSageCondition.EIGHT", eight),
    Handler::value("TellahGreatSage.TellahGreatSageValue", spent),
];

/// `getValue("spellCast")`'s mana spent to cast it (0 without the trigger's spell)
fn mana_spent(game: &Game) -> i32 {
    let item = game.resolving_item.as_ref().or(game.checking_item.as_ref());
    item.and_then(|i| i.cast_spell).map_or(0, |s| game.obj(s).mana_spent as i32)
}

fn four(game: &Game, _you: Seat, _source: ObjId) -> bool {
    mana_spent(game) >= 4
}

fn eight(game: &Game, _you: Seat, _source: ObjId) -> bool {
    mana_spent(game) >= 8
}

fn spent(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    mana_spent(game)
}
