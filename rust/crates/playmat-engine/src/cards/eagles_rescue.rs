//! Eagle's Rescue: "Enchant creature. Enchanted creature gets +2/+2 and has flying. {2}{W/U}{W/U}: Return this card
//! from your graveyard to the battlefield attached to target creature you control with power 1 or less. Activate
//! only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EaglesRescue.EaglesRescueEffect", apply)];

/// Still in the graveyard: it enters attached to the target (`"attachTo:" + auraId`), not asked where (as Gryff's
/// Boon)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(aura).zone != Zone::Graveyard || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    game.attach_entering = Some((aura, to, game.obj(to).zcc));
    put_onto_battlefield(game, d, aura, cx.controller());
    game.attach_entering = None;
    if d.stopped() {
        return None;
    }
    Some(())
}
