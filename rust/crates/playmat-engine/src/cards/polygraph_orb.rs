//! Polygraph Orb: "When Polygraph Orb enters, look at the top four cards of your library. Put two of them into your
//! hand and the rest into your graveyard. You lose 2 life. {2}, {T}, Collect evidence 3: Each opponent loses 3 life
//! unless they discard a card or sacrifice a creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PolygraphOrb.PolygraphOrbEffect", apply)];

/// Each opponent in turn order chooses up to one card in hand to discard (`TargetDiscard(0, 1)`, one
/// `HumanPlayer.choose`, asked even with none), else up to one creature they control to sacrifice (the same); then each discards, sacrifices or loses 3 life
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let opponents: Vec<Seat> = (0..n).map(|k| (you + k) % n).filter(|p| *p != you && !game.player(*p).lost).collect();
    let mut chosen: Vec<(Seat, Option<ObjId>)> = Vec::new();
    for &p in &opponents {
        let hand = game.player(p).hand.clone();
        let rule = "discard a card";
        if let Some(&c) = choose_cards(game, d, p, &hand, &hand, 0, 1, rule)?.first() {
            chosen.push((p, Some(c)));
            continue;
        }
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        if let Some(&c) = choose_cards(game, d, p, &creatures, &creatures, 0, 1, "sacrifice a creature")?.first() {
            chosen.push((p, Some(c)));
            continue;
        }
        chosen.push((p, None));
    }
    for (p, c) in chosen {
        if let Some(c) = c {
            match game.obj(c).zone {
                Zone::Hand => {
                    discard_card(game, p, c);
                    game.discarded_cards();
                    continue;
                }
                Zone::Battlefield => {
                    sacrifice(game, c);
                    continue;
                }
                _ => {}
            }
        }
        lose_life(game, p, 3);
    }
    Some(())
}
