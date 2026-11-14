//! Sword of Once and Future: "… Then you may cast an instant or sorcery spell with mana value 2 or less from your
//! graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SwordOfOnceAndFuture.SwordOfOnceAndFutureCastEffect", apply)];

/// `castSpellWithAttributesForFree` over your graveyard with "instant or sorcery, mana value less than 3": one
/// castable card is taken as it is, more are chosen among (up to one); `chooseUse`, then the cast, exiled instead of
/// put into your graveyard as it leaves the stack (`SwordOfOnceAndFutureReplacementEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let options: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            (o.is(CardType::Instant) || o.is(CardType::Sorcery)) && o.def.mana_value() < 3
        })
        .collect();
    let card = match options.len() {
        0 => return Some(()),
        1 => options[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if choose_use(game, d, you, &message)? {
        let cast = cast_free(game, d, you, card);
        if d.stopped() {
            return None;
        }
        if cast {
            game.obj_mut(card).flashback = true;
        }
    }
    Some(())
}
