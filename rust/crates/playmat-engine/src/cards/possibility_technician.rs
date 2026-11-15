//! Possibility Technician: "Whenever this creature or another Kavu you control enters, exile the top card of your
//! library. For as long as that card remains exiled, you may play it if you control a Kavu."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PossibilityTechnician.PossibilityTechnicianEffect", apply)];

/// The top card into exile (no zone of the source's), then `makeCardPlayable` for as long as it stays
/// (`Duration.Custom`), any way to play it, with its condition: you control a Kavu
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = *game.player(you).library.first()?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let o = game.obj_mut(card);
    o.may_play = Some((you, u32::MAX));
    o.may_play_end_step = false;
    o.may_play_while = None;
    o.may_play_if = Some(controls_a_kavu);
    Some(())
}

/// `PermanentsOnTheBattlefieldCondition(FilterControlledPermanent(SubType.KAVU))`
fn controls_a_kavu(game: &Game, seat: Seat) -> bool {
    game.battlefield.iter().any(|p| game.obj(*p).controller == seat && game.has_subtype(*p, "Kavu"))
}
