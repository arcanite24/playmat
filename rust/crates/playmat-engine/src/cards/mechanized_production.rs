//! Mechanized Production: "At the beginning of your upkeep, create a token that's a copy of enchanted artifact. Then
//! if you control eight or more artifacts with the same name as one another, you win the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MechanizedProduction.MechanizedProductionEffect", apply)
    .effects(&["new CreateTokenCopyTargetEffect()"])];

/// The copy of what the Aura is attached to, then the names of the artifacts you control counted
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let Some(artifact) = game.attached(src) else { return Some(()) };
    if game.obj(artifact).zone == Zone::Battlefield {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(artifact)]);
        if d.stopped() {
            return None;
        }
    }
    let you = cx.controller();
    let mut names: Vec<(String, u32)> = Vec::new();
    for p in game.battlefield.iter() {
        let o = game.obj(*p);
        if o.controller != you || !game.has_type(*p, CardType::Artifact) {
            continue;
        }
        match names.iter_mut().find(|(n, _)| *n == o.def.name) {
            Some((_, k)) => *k += 1,
            None => names.push((o.def.name.clone(), 1)),
        }
    }
    if names.iter().any(|(_, k)| *k > 7) {
        win_game(game, you);
    }
    Some(())
}
