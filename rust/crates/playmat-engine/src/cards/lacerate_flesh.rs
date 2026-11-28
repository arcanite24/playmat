//! Lacerate Flesh: "Lacerate Flesh deals 4 damage to target creature. Create a number of Blood tokens equal to the
//! amount of excess damage dealt to that creature this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LacerateFlesh.LacerateFleshEffect", apply)
    .effects(&["new CreateTokenEffect(new BloodToken())"])];

/// 4 damage with excess (`damageWithExcess`), then that many Blood tokens
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let excess = super::goblin_negotiation::damage_with_excess(game, cx.source_id(), o, 4);
    if excess > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = crate::card::Amount::Fixed(excess);
        }
        cx.run_on(game, d, &[e], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
