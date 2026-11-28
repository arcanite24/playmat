//! Trostani, Selesnya's Voice: "Whenever another creature you control enters, you gain life equal to that
//! creature's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TrostaniSelesnyasVoice.TrostaniSelesnyasVoiceTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(Zone.BATTLEFIELD, new TrostaniSelesnyasVoiceEffect(), StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE, false, SetTargetPointer.PERMANENT)",
        any,
    ),
    Handler::effect("TrostaniSelesnyasVoice.TrostaniSelesnyasVoiceEffect", gain_toughness),
];

/// `checkTrigger` is the base trigger's: another creature you control entered
fn any(_: &Game, _: &StackItem) -> bool {
    true
}

/// `getPermanentOrLKIBattlefield` of the creature that entered: its toughness now, or as it last was on the
/// battlefield (no life when nothing's known of it)
pub(super) fn gain_toughness(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((Target::Obj(o), zcc)) = cx.item.targets.first().zip(cx.item.target_zcc.first()).and_then(|(ts, zs)| {
        ts.first().copied().zip(zs.first().copied())
    }) else {
        return Some(());
    };
    let ob = game.obj(o);
    let toughness = if ob.zone == Zone::Battlefield && ob.zcc == zcc {
        Some(game.power_toughness(o).1)
    } else if ob.zcc == zcc + 1 {
        ob.toughness_lki
    } else {
        None
    };
    if let Some(n) = toughness {
        gain_life(game, cx.controller(), n);
    }
    Some(())
}
