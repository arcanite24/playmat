//! Tymaret Calls the Dead: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I, II — Mill three cards. Then you may exile a creature or enchantment card from your graveyard. If you do, create
//! a 2/2 black Zombie creature token. III — You gain X life and scry X, where X is the number of Zombies you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TymaretCallsTheDead.TymaretCallsTheDeadFirstEffect", apply)
    .effects(&["new MillCardsControllerEffect(3)"])
    .tokens(&["ZombieToken"])];

/// Three cards milled; with a creature or enchantment card in your graveyard, "Exile a creature or enchantment card
/// from your graveyard?", then one chosen (not targeted) is exiled and a Zombie made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let graveyard = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) || game.obj(*c).is(CardType::Enchantment))
        .collect();
    if possible.is_empty()
        || !choose_use(game, d, you, "Exile a creature or enchantment card from your graveyard?")?
    {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &graveyard, &possible, 1, 1, &rule)?.first() else {
        return Some(());
    };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    Some(())
}
