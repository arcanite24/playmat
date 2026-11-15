//! Avatar's Wrath: "Choose up to one target creature, then airbend all other creatures. Until your next turn, your
//! opponents can't cast spells from anywhere other than their hand. Exile Avatar's Wrath."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AvatarsWrath.AvatarsWrathAirbendEffect", airbend),
    Handler::rule("AvatarsWrath.AvatarsWrathRuleEffect", Rule::CantCast(not_from_hand)).args(&["until_your_next_turn"]),
];

/// `AirbendTargetEffect` on every creature but the target (if it's still legal), in battlefield order
fn airbend(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let chosen = cx.obj(0);
    let others: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| Some(*o) != chosen && game.has_type(*o, CardType::Creature))
        .map(Target::Obj)
        .collect();
    if others.is_empty() {
        return Some(());
    }
    cx.run_on(game, d, &[EffectDef::AirbendTargetEffect], others);
    Some(())
}

/// `CAST_SPELL` by an opponent of a card that isn't in a hand (or, being cast, didn't come from one)
fn not_from_hand(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> bool {
    let o = game.obj(id);
    let from = if o.zone == Zone::Stack { o.cast_from } else { Some(o.zone) };
    cx.opponent(seat) && from != Some(Zone::Hand)
}
