//! Halo-Charged Skaab: "When Halo-Charged Skaab enters, each player mills two cards. Then you may put an instant,
//! sorcery, or battle card from your graveyard on top of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HaloChargedSkaab.HaloChargedSkaabEffect", apply)];

/// `player.choose(TargetCardInYourGraveyard(0, 1, filter, true))` rounds, the card put on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.is(CardType::Instant) || o.is(CardType::Sorcery) || o.is(CardType::Battle)
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    if let Some(c) = choose_cards_rounds(game, d, you, &possible, 0, 1)?.first() {
        to_library(game, *c, true);
    }
    Some(())
}
