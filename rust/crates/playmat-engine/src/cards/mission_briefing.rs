//! Mission Briefing: "Surveil 2, then choose an instant or sorcery card in your graveyard. You may cast it this turn.
//! If that spell would be put into your graveyard, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MissionBriefing.MissionBriefingEffect", apply)];

/// Surveil 2; then one instant or sorcery card of your graveyard (not targeted, and asked even with only one):
/// `MayCastTargetCardEffect(EndOfTurn, true)`, `makeCardPlayable` this turn as a spell only, and
/// `ThatSpellGraveyardExileReplacementEffect` for it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, &[EffectDef::SurveilEffect { amount: crate::card::Amount::Fixed(2) }]);
    if d.stopped() {
        return None;
    }
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    let Some(&card) = choose_cards(game, d, you, &cards, &cards, 0, 1, "an instant or sorcery card")?.first() else {
        return Some(());
    };
    let turn = game.turn;
    let o = game.obj_mut(card);
    o.may_play = Some((you, turn));
    o.cast_only = true;
    game.exile_if_graveyard.push(card);
    Some(())
}
