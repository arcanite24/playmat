//! Aurelia, Exemplar of Justice: "At the beginning of combat on your turn, choose up to one target creature you
//! control. Until end of turn, that creature gets +2/+0, gains trample if it's red, and gains vigilance if it's
//! white."

use super::*;
use crate::card::{Color, Duration};

pub const HANDLERS: &[Handler] = &[Handler::effect("AureliaExemplarOfJustice.AureliaExemplarOfJusticeEffect", apply)];

/// Its colors as it resolves pick the abilities (each a `GainAbilityTargetEffect` on the target)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let colors = game.colors(o);
    let mut effects = vec![EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(2),
        toughness: crate::card::Amount::Fixed(0),
        duration: Duration::EndOfTurn,
    }];
    if colors.contains(&Color::R) {
        effects.push(EffectDef::GainAbilityTargetEffect {
            ability: Box::new(AbilityDef::TrampleAbility),
            duration: Duration::EndOfTurn,
        });
    }
    if colors.contains(&Color::W) {
        effects.push(EffectDef::GainAbilityTargetEffect {
            ability: Box::new(AbilityDef::VigilanceAbility),
            duration: Duration::EndOfTurn,
        });
    }
    cx.run_on(game, d, &effects, vec![Target::Obj(o)]);
    Some(())
}
