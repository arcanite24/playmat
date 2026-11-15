//! Deicide: "Exile target enchantment. If the exiled card is a God card, search its controller's graveyard, hand,
//! and library for any number of cards with the same name as that card and exile them, then that player shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Deicide.DeicideExileEffect", apply)];

/// The enchantment into exile; then, if the card in exile is a God card, `applySearchAndExile` with its name for
/// the player who controlled it (the graveyard's choice optional)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let controller = game.obj(target).controller;
    game.move_to(target, Zone::Exile);
    // a token leaves no card in exile
    let o = game.obj(target);
    if o.zone != Zone::Exile || o.token || !game.has_subtype(target, "God") {
        return Some(());
    }
    let name = game.obj(target).def.name.clone();
    super::search_for_card_name_and_exile::apply_search_and_exile(game, d, cx.controller(), controller, &name, usize::MAX, false)
}
