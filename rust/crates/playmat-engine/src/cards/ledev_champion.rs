//! Ledev Champion: "Whenever Ledev Champion attacks, you may tap any number of untapped creatures you control. Ledev
//! Champion gets +1/+1 until end of turn for each creature tapped this way. {3}{G}{W}: Create a 1/1 white Soldier
//! creature token with lifelink."

use super::*;
use crate::card::Duration;

pub const HANDLERS: &[Handler] = &[Handler::effect("LedevChampion.LedevChampionEffect", apply)];

/// `TargetPermanent(0, MAX, untapped creatures you control, true).choose`: `TargetImpl.choose` rounds; each chosen
/// still on the battlefield is tapped, and with one or more tapped, +N/+N until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && !game.obj(*o).tapped
        })
        .collect();
    let chosen = choose_cards_rounds(game, d, you, &possible, 0, usize::MAX)?;
    let mut n = 0;
    for c in chosen {
        if game.obj(c).zone == Zone::Battlefield {
            tap(game, c);
            n += 1;
        }
    }
    if n > 0 {
        let boost =
            EffectDef::BoostSourceEffect { power: Amount::Fixed(n), toughness: Amount::Fixed(n), duration: Duration::EndOfTurn };
        cx.run(game, d, &[boost]);
    }
    Some(())
}
