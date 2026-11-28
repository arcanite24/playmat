//! Scourge of Valkas: "Whenever this creature or another Dragon you control enters, it deals X damage to any target,
//! where X is the number of Dragons you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScourgeOfValkas.ScourgeOfValkasDamageEffect", apply)
    .filters(&[r#"new FilterControlledPermanent(SubType.DRAGON, "")"#])];

/// The entering Dragon (`permanentEnteringBattlefield`) deals the damage; nothing without Dragons
pub(super) fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(dragon) = cx.item.entering else { return Some(()) };
    let n = matching(game, &cx.filters[0], cx.controller(), cx.source_id()).len() as i32;
    if n < 1 {
        return Some(());
    }
    if let Some(t) = cx.target(0) {
        deal_damage(game, dragon, t, n, false);
    }
    Some(())
}
