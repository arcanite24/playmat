//! Lifecraft Awakening: "Put X +1/+1 counters on target artifact you control. If it isn't a creature or Vehicle, it
//! becomes a 0/0 Construct artifact creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LifecraftAwakening.LifecraftAwakeningEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(0,0,\"0/0 Construct artifact creature\",SubType.CONSTRUCT).withType(CardType.ARTIFACT),false,false,Duration.WhileOnBattlefield)",
])];

/// the target, if it isn't a creature or a Vehicle now, becomes the Construct until it leaves (`Duration.Custom`, a resolved "while on the battlefield")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.obj(0)?;
    if game.obj(id).zone != Zone::Battlefield {
        return Some(());
    }
    if !game.has_type(id, CardType::Creature) && !game.has_subtype(id, "Vehicle") {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}
