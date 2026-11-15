//! Illusory Gains: "Enchant creature. You control enchanted creature. Whenever a creature enters under an
//! opponent's control, attach Illusory Gains to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IllusoryGains.IllusoryGainsEffect", apply)];

/// the Aura on the battlefield and still attached to a permanent: it moves to the creature that entered (the
/// trigger's pointer), and the control it gives follows it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(src).zone != Zone::Battlefield || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    if game.attached(src).is_none() {
        return Some(());
    }
    let zcc = game.obj(to).zcc;
    game.attach_to(src, to, zcc);
    refresh_becomes(game);
    Some(())
}
