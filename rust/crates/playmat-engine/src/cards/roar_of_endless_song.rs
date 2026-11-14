//! Roar of Endless Song: "III — Double the power and toughness of each creature you control until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RoarOfEndlessSong.RoarOfEndlessSongEffect", apply)];

/// a `BoostTargetEffect` of each creature's own power and toughness, read one creature at a time
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| game.obj(*b).controller == you && game.has_type(*b, CardType::Creature))
        .collect();
    for o in mine {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let (p, t) = game.power_toughness(o);
        let e = EffectDef::BoostTargetEffect {
            power: crate::card::Amount::Fixed(p),
            toughness: crate::card::Amount::Fixed(t),
            duration: crate::card::Duration::EndOfTurn,
        };
        cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
