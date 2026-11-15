//! `GainControlAllUntapGainHasteEffect(filter)`: "gain control of all [filter] until end of turn. Untap them. They
//! gain haste until end of turn" (Mob Rule, Broadcast Takeover, Mutinous Massacre, Rowan, Fearless Sparkmage). The
//! permanents are the ones of the filter as it resolves (`PermanentReferenceInCollectionPredicate`).

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("GainControlAllUntapGainHasteEffect", apply)
    .effects(&[
        "new GainControlTargetEffect(Duration.EndOfTurn)",
        "new UntapTargetEffect()",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
    ])
    .args(&["filter"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let objs: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).zone == Zone::Battlefield && game.filter_matches(filter, you, Some(cx.source_id()), *o))
        .map(Target::Obj)
        .collect();
    if objs.is_empty() {
        return Some(());
    }
    for e in cx.effects {
        cx.run_on(game, d, std::slice::from_ref(e), objs.clone());
    }
    Some(())
}
