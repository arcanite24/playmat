//! Final Showdown: "Spree … + {1} — Choose a creature you control. It gains indestructible until end of turn."

use super::*;
use crate::card::Duration;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinalShowdown.FinalShowdownEffect", apply)];

/// With a creature you control: `choose(TargetControlledCreaturePermanent, notTarget)`, then
/// `GainAbilityTargetEffect(IndestructibleAbility, EndOfTurn)` on it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if mine.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(o) = choose_cards(game, d, you, &mine, &mine, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let e = EffectDef::GainAbilityTargetEffect {
        ability: Box::new(AbilityDef::IndestructibleAbility),
        duration: Duration::EndOfTurn,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    Some(())
}
