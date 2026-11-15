//! Portal of Sanctuary: "{1}, {T}: Return target creature you control and each Aura attached to it to their owners'
//! hands. Activate only during your turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PortalOfSanctuary.PortalOfSanctuaryEffect", apply)];

/// the creature, then the Auras attached to it, to their owners' hands (`moveCards` to `HAND`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let mut cards = vec![o];
    cards.extend(game.battlefield.iter().copied().filter(|a| {
        game.obj(*a).attached_to.is_some_and(|(to, _)| to == o) && game.has_subtype(*a, "Aura")
    }));
    put_cards(game, d, cx.controller(), &cards, crate::card::PutCards::Hand);
    Some(())
}
