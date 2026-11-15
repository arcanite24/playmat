//! Fortified Beachhead: "As Fortified Beachhead enters the battlefield, you may reveal a Soldier card from your hand.
//! Fortified Beachhead enters the battlefield tapped unless you revealed a Soldier card this way or you control a
//! Soldier. {T}: Add {W} or {U}. {5}, {T}: Soldiers you control get +1/+1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FortifiedBeachhead.FortifiedBeachheadEffect", apply)];

/// As it enters: `TargetCardInHand(0, 1, Soldier card)` (nothing to choose, nothing asked); one chosen is revealed
/// and it enters untapped; else untapped too if you control a Soldier, otherwise tapped (`setTapped`, no trigger)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let land = cx.source_id();
    let soldiers: Vec<ObjId> =
        game.player(you).hand.iter().copied().filter(|c| *c != land && game.has_subtype(*c, "Soldier")).collect();
    if !soldiers.is_empty() {
        let chosen = choose_cards(game, d, you, &soldiers, &soldiers, 0, 1, "to reveal")?;
        if let Some(&card) = chosen.first() {
            let zcc = game.obj(land).zcc;
            game.revealed.push(crate::state::Revealed { source: land, zcc, paren: false, cards: vec![(card, Zone::Hand)] });
            return Some(());
        }
    }
    let soldier = game
        .battlefield
        .iter()
        .any(|p| *p != land && game.obj(*p).controller == you && !game.obj(*p).phased_out && game.has_subtype(*p, "Soldier"));
    if !soldier {
        game.obj_mut(land).tapped = true;
    }
    Some(())
}
