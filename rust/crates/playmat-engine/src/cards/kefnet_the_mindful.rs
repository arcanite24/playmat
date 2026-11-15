//! Kefnet the Mindful: "Flying, indestructible. Kefnet the Mindful can't attack or block unless you have seven or
//! more cards in your hand. {3}{U}: Draw a card, then you may return a land you control to its owner's hand."

use super::*;
use crate::card::{Filter, Pred};

pub const HANDLERS: &[Handler] = &[
    Handler::restriction(
        "KefnetTheMindful.KefnetTheMindfulRestrictionEffect",
        applies,
        &[Restrict::CantAttack(no_attack), Restrict::CantBlock(no_block)],
    ),
    Handler::effect("KefnetTheMindful.KefnetTheMindfulEffect", apply),
];

/// the source itself, while its controller has fewer than seven cards in hand
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source && game.player(cx.controller).hand.len() < 7
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}

/// A card drawn, then "Return a land you control to its owner's hand?" and
/// `ReturnToHandChosenControlledPermanentEffect` of a land
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    if choose_use(game, d, you, "Return a land you control to its owner's hand?")? {
        let filter = Filter(vec![Pred::Type { t: CardType::Land }]);
        cx.run(game, d, &[EffectDef::ReturnToHandChosenControlledPermanentEffect { filter, amount: 1 }]);
    }
    (!d.stopped()).then_some(())
}
