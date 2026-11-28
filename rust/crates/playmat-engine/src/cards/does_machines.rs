//! Does Machines: "When this Class enters, mill two cards, draw two cards, then discard two cards.
//! {1}{U}: Level 2. When this Class becomes level 2, return up to two target artifact cards from your graveyard to
//! your hand. {4}{U}: Level 3. At the beginning of combat on your turn, put three +1/+1 counters on target artifact
//! you control. If it isn't a creature, it becomes a 0/0 Robot creature in addition to its other types."

use super::*;

// (`withKeepPriorTypes(true)` keeps every prior type and subtype, as "it's still a land" does)
pub const HANDLERS: &[Handler] = &[Handler::effect("DoesMachines.DoesMachinesEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(0,0,\"0/0 Robot creature\").withSubType(SubType.ROBOT), false, true, Duration.Custom)",
])];

/// The target, still there and not a creature, becomes a 0/0 Robot creature in addition to its other types until it
/// leaves the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    if game.has_type(o, CardType::Creature) {
        return Some(());
    }
    cx.run_on(game, d, cx.effects, vec![Target::Obj(o)]);
    (!d.stopped()).then_some(())
}
