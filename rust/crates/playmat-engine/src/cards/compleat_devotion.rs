//! Compleat Devotion: "Target creature you control gets +2/+2 until end of turn. If that creature has toxic, draw a
//! card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CompleatDevotion.CompleatDevotionEffect", apply)];

/// With the target still on the battlefield and a toxic ability (printed or granted), its controller draws a card
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let toxic = |a: &AbilityDef| matches!(a, AbilityDef::ToxicAbility { .. });
    if game.obj(o).def.abilities.iter().any(toxic) || game.granted_abilities(o).iter().any(toxic) {
        let controller = game.obj(o).controller;
        draw(game, controller, 1);
    }
    Some(())
}
