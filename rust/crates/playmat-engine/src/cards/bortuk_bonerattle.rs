//! Bortuk Bonerattle: "Domain — When Bortuk Bonerattle enters, if you cast it, choose target creature card in your
//! graveyard. Return that card to the battlefield if its mana value is less than or equal to the number of basic
//! land types among lands you control. Otherwise, put it into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BortukBonerattle.BortukBonerattleCondition", low_enough)];

/// the resolving trigger's first target, a card with mana value at most your domain count (`DomainValue.REGULAR`)
fn low_enough(game: &Game, you: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let Some(Target::Obj(card)) = item.targets.first().and_then(|t| t.first()).copied() else { return false };
    let domain = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .filter(|t| {
            game.battlefield.iter().any(|l| {
                game.obj(*l).controller == you && game.has_type(*l, CardType::Land) && game.has_subtype(*l, t)
            })
        })
        .count() as i32;
    (game.obj(card).def.mana_value() as i32) <= domain
}
