//! Macabre Mockery: "Put target creature card from an opponent's graveyard onto the battlefield under your control.
//! It gets +2/+0 and gains haste until end of turn. Sacrifice it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MacabreMockery.MacabreMockeryEffect", apply)];

/// The card onto the battlefield under your control; then haste for as long as it stays (`Duration.Custom`), +2/+0
/// until end of turn, and the delayed "sacrifice <it>"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(card).zcc;
    Arc::make_mut(&mut game.grants).push(crate::state::Grant {
        target: card,
        zcc,
        ability: AbilityDef::HasteAbility,
        until_end_of_turn: false,
        until_turn_of: None,
    });
    boost(game, card, 2, 0, crate::card::Duration::EndOfTurn);
    let delayed = crate::state::Delayed {
        rule: format!("At the beginning of the next end step, sacrifice {}.", log::object(game, card)),
        sacrifice: vec![(card, zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
