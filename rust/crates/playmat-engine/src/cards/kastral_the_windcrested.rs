//! Kastral, the Windcrested: "Flying. Whenever one or more Birds you control deal combat damage to a player, choose
//! one — You may put a Bird creature card from your hand or graveyard onto the battlefield with a finality counter
//! on it; or put a +1/+1 counter on each Bird you control; or draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KastralTheWindcrested.KastralTheWindcrestedEffect", apply)];

/// the Bird creature cards in your hand, then your graveyard; you choose one, it enters with a finality counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let bird = |c: &ObjId| game.obj(*c).def.is(CardType::Creature) && game.has_subtype(*c, "Bird");
    let cards: Vec<ObjId> = game
        .player(you)
        .hand
        .iter()
        .chain(game.player(you).graveyard.iter())
        .copied()
        .filter(bird)
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?;
    let Some(&c) = chosen.first() else { return Some(()) };
    if matches!(game.obj(c).zone, Zone::Hand | Zone::Graveyard) {
        enter_battlefield(game, d, c, you, false);
        if game.obj(c).zone == Zone::Battlefield {
            add_counters_by(game, c, "finality", 1, Some(you));
        }
    }
    Some(())
}
