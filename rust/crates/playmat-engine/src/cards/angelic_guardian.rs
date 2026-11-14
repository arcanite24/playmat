//! Angelic Guardian: "Whenever one or more creatures you control attack, they gain indestructible until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AngelicGuardian.AngelicGuardianGainEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"])];

/// every attacking creature you control as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = attackers(game)
        .into_iter()
        .filter(|o| {
            game.obj(*o).zone == Zone::Battlefield
                && game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
        })
        .collect();
    for o in mine {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
