//! Ajani's Chosen: "Whenever an enchantment you control enters, create a 2/2 white Cat creature token. If that
//! enchantment is an Aura, you may attach it to the token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AjanisChosen.AjanisChosenEffect", apply)
    .effects(&["new CreateTokenEffect(new CatToken())"])];

/// The Cat; then an Aura (the trigger's permanent) still attached to a permanent may move to each Cat made, when its
/// "enchant" target could choose the token ("Attach <name> to the token ?")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let Some(aura) = cx.obj(0) else { return Some(()) };
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        if game.obj(aura).zone != Zone::Battlefield || !game.has_subtype(aura, "Aura") {
            return Some(());
        }
        if game.attached(aura).is_none_or(|a| game.obj(a).zone != Zone::Battlefield) {
            continue;
        }
        let you = cx.controller();
        let can = game.obj(aura).def.spell.targets.first().is_none_or(|def| {
            let (_, objs) = possible_targets(game, you, def, Some(aura));
            objs.contains(&t)
        });
        let message = format!("Attach {} to the token ?", game.obj(aura).name());
        if can && choose_use(game, d, you, &message)? {
            let zcc = game.obj(t).zcc;
            game.attach_to(aura, t, zcc);
        }
    }
    Some(())
}
