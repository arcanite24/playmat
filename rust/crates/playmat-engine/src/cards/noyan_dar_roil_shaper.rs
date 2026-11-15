//! Noyan Dar, Roil Shaper: "Whenever you cast an instant or sorcery spell, you may put three +1/+1 counters on target
//! land you control. If you do, that land becomes a 0/0 Elemental creature with haste that's still a land."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NoyanDarRoilShaper.NoyanDarEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(0,0,\"0/0 Elemental creature with haste\",SubType.ELEMENTAL).withAbility(HasteAbility.getInstance()), false, true, Duration.EndOfGame)",
])];

/// the land becomes the Elemental for good (`Duration.EndOfGame`), then gets the three counters, as Java orders them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let land = cx.obj(0)?;
    if game.obj(land).zone != Zone::Battlefield {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    put_counters(game, land, "+1/+1", 3, cx.controller());
    Some(())
}
