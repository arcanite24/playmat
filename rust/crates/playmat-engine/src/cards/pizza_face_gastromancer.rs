//! Pizza Face, Gastromancer: "When Pizza Face enters, create a Food token. Disappear — At the beginning of your end
//! step, if a permanent you controlled left the battlefield this turn, put three +1/+1 counters on up to one other
//! target artifact or creature. If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other
//! types. {10}, {T}, Sacrifice Pizza Face: You gain 15 life."

use super::*;
use crate::card::{Duration, TokenDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("PizzaFaceGastromancer.PizzaFaceGastromancerEffect", apply)];

/// the target, if it isn't a creature now, becomes a 0/0 Mutant creature for as long as it stays
/// (`Duration.Custom`), keeping its prior types
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.has_type(o, CardType::Creature) {
        return Some(());
    }
    let token = TokenDef {
        types: vec![CardType::Creature],
        subtypes: vec!["Mutant".into()],
        power: 0,
        toughness: 0,
        ..Default::default()
    };
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: Duration::WhileOnBattlefield,
        lose_abilities: false,
        still_land: true,
        named: None,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    Some(())
}
