//! Winter Soldier, Icy Assassin: "Vigilance, menace. Winter Soldier gets +2/+0 for each Equipment attached to him.
//! {3}{W}{B}: Return this card from your graveyard to the battlefield with a finality counter on him. Then you may
//! attach an Equipment you control to him."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("WinterSoldierIcyAssassin.WinterSoldierIcyAssassinAttachEffect", apply)
        .filters(&["filter"])];

/// `getSourcePermanentIfItStillExists`: the source as it was when the ability was activated (in the graveyard), so
/// once it has returned (a new object) Java finds no source and the effect does nothing; with it still the same
/// object on the battlefield, up to one Equipment you control (`player.choose`, not targeted) attached to it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let src = cx.source_id();
    let possible = matching(game, &cx.filters[0], you, src);
    let chosen = super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)?;
    if let Some(&equipment) = chosen.first() {
        if game.obj(equipment).zone == Zone::Battlefield && game.attached(equipment) != Some(src) {
            let zcc = game.obj(src).zcc;
            game.attach_to(equipment, src, zcc);
        }
    }
    Some(())
}
