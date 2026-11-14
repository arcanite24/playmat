//! Curse of the Swine: "Exile X target creatures. For each creature exiled this way, its controller creates a 2/2
//! green Boar creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("CurseOfTheSwine.CurseOfTheSwineEffect", apply).tokens(&["Boar2Token"])];

/// The targets exiled together; each one that ended up in exile (a token: each one that was on the battlefield)
/// counts for the player who controlled it, and each of those players creates that many Boars. Java keeps the
/// counts in a `HashMap` by player id, so its order between two players isn't reproducible: here, by seat.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut counts: Vec<(Seat, i32)> = Vec::new();
    let add = |counts: &mut Vec<(Seat, i32)>, seat: Seat| match counts.iter_mut().find(|(s, _)| *s == seat) {
        Some((_, n)) => *n += 1,
        None => counts.push((seat, 1)),
    };
    let mut moved = Vec::new();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let controller = game.obj(o).controller;
        if game.obj(o).token {
            add(&mut counts, controller);
        }
        moved.push((o, controller));
    }
    for (o, _) in &moved {
        game.move_to(*o, Zone::Exile);
    }
    for (o, controller) in moved {
        if !game.obj(o).token && game.obj(o).zone == Zone::Exile {
            add(&mut counts, controller);
        }
    }
    counts.sort_by_key(|(s, _)| *s);
    for (seat, n) in counts {
        let e = EffectDef::CreateTokenEffect {
            token: cx.tokens[0].clone(),
            amount: crate::card::Amount::Fixed(n),
            tapped_and_attacking: false,
            tapped: false,
            attacking: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            haste_until_end_of_turn: false,
            also: None,
            attach_source: None,
        };
        cx.run_as(game, d, &[e], seat);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
