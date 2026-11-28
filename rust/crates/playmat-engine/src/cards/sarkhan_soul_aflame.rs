//! Sarkhan, Soul Aflame: "Whenever a Dragon you control enters, you may have Sarkhan, Soul Aflame become a copy of it
//! until end of turn, except its name is Sarkhan, Soul Aflame and it's legendary in addition to its other types."

use super::*;
use crate::state::Zone;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("SarkhanSoulAflame.SarkhanSoulAflameEffect", apply)];

/// `getSourcePermanentIfItStillExists` and the Dragon that entered (`permanentEnteringBattlefield`, as it is now):
/// `copyPermanent(Duration.EndOfTurn, …)` with the applier's name and legendary
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(of) = cx.item.entering else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let mut copy = (*game.obj(of).copiable()).clone();
    copy.name = "Sarkhan, Soul Aflame".into();
    if !copy.supertypes.iter().any(|s| s == "LEGENDARY") {
        copy.supertypes.push("LEGENDARY".into());
    }
    become_copy_until_end_of_turn(game, src, Arc::new(copy));
    Some(())
}
