//! Tawnos's Tinkering: "Put two +1/+1 counters on target artifact, creature, or land you control. Untap that
//! permanent. If it isn't a creature, it becomes a 0/0 creature in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TawnossTinkering.TawnossTinkeringEffect", apply)
    .effects(&["new BecomesCreatureTargetEffect(new CreatureToken(0,0),false,false,Duration.WhileOnBattlefield)"])];

/// the target, if it isn't a creature now, becomes a 0/0 creature keeping its other types until it leaves
/// (`Duration.Custom` with a `FixedTarget`, a resolved "while on the battlefield")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.obj(0)?;
    if game.obj(id).zone != Zone::Battlefield || game.has_type(id, CardType::Creature) {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
