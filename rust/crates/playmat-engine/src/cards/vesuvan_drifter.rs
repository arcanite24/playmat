//! Vesuvan Drifter: "Flying. You may look at the top card of your library any time. At the beginning of each combat,
//! you may reveal the top card of your library. If you reveal a creature card this way, Vesuvan Drifter becomes a copy
//! of that card until end of turn, except it has flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VesuvanDrifter.VesuvanDrifterEffect", apply)];

/// The top card revealed; a creature card, with the Drifter still the permanent that triggered, makes it a copy of
/// that card until end of turn (`copyPermanent(EndOfTurn, new PermanentCard(card), …)`), its applier adding flying
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let Some(&top) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(src).zcc;
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: vec![(top, Zone::Library)] });
    if !game.has_type(top, CardType::Creature) || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let mut copy = (*game.obj(top).def).clone();
    if !copy.abilities.iter().any(|a| matches!(a, AbilityDef::FlyingAbility)) {
        copy.abilities.push(AbilityDef::FlyingAbility);
    }
    become_copy_until_end_of_turn(game, src, Arc::new(copy));
    Some(())
}
