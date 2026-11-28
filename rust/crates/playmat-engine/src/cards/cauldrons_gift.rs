//! Cauldron's Gift: "Adamant — If at least three black mana was spent to cast this spell, mill four cards. You may
//! choose a creature card in your graveyard. If you do, return it to the battlefield with an additional +1/+1 counter
//! on it."

use super::*;
use crate::card::{Amount, Color};

pub const HANDLERS: &[Handler] = &[Handler::effect("CauldronsGift.CauldronsGiftEffect", apply)];

/// `AdamantCondition.BLACK`, the mill; then with a creature card in your graveyard, "Choose a creature card in your
/// graveyard to return to the battlefield?", the card chosen (`Player.choose`, not targeted) and put onto the
/// battlefield under your control with a +1/+1 counter as it enters (`setEnterWithCounters`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = game.obj(cx.item.card);
    let adamant = cx.item.ability.is_none() && spell.zone == Zone::Stack && spell.spent_colors[Color::B.index()] >= 3;
    if adamant {
        cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: Amount::Fixed(4) }]);
        if d.stopped() {
            return None;
        }
    }
    let creatures: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    if creatures.is_empty()
        || !choose_use(game, d, you, "Choose a creature card in your graveyard to return to the battlefield?")?
    {
        return Some(());
    }
    let shown = game.player(you).graveyard.clone();
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &shown, &creatures, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: false, counters: None, attacking: false };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 1));
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    Some(())
}
