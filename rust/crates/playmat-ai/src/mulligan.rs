//! Opening hands: whether the AI keeps or mulligans, and which cards go to the bottom (the London mulligan).
//! Used by the Rust host for its AI seats and by the benches, so both play the same way.

use playmat_engine::card::CardType;
use playmat_engine::rules;
use playmat_engine::state::{Game, ObjId, Seat, Zone};

/// Whether the AI mulligans `seat`'s hand after `taken` mulligans that cost a card (free ones don't count): two to
/// five lands are kept on seven, one to five on six, and anything on five or fewer.
pub fn rule_wants(game: &Game, seat: Seat, taken: usize) -> bool {
    let hand = &game.player(seat).hand;
    let lands = hand.iter().filter(|id| game.obj(**id).is(CardType::Land)).count();
    match taken {
        0 => !(2..=5).contains(&lands),
        1 => !(1..=5).contains(&lands),
        _ => false,
    }
}

/// The card the rule puts on the bottom: a land when the hand has more than three, else its most expensive spell.
pub fn rule_bottom(game: &Game, seat: Seat) -> ObjId {
    let hand = &game.player(seat).hand;
    let lands: Vec<ObjId> = hand.iter().copied().filter(|id| game.obj(*id).is(CardType::Land)).collect();
    if lands.len() > 3 {
        return lands[0];
    }
    hand.iter()
        .copied()
        .filter(|id| !game.obj(*id).is(CardType::Land))
        .max_by_key(|id| game.obj(*id).def.mana_value())
        .unwrap_or(hand[0])
}

/// The London mulligan for seats that all decide by `wants` and `bottom` (the benches; the host runs its own loop,
/// with prompts for people): the starting player first, `free` mulligans that keep the hand's size.
pub fn london(
    game: &mut Game,
    first: Seat,
    free: usize,
    wants: &mut dyn FnMut(&Game, Seat, usize) -> bool,
    bottom: &mut dyn FnMut(&Game, Seat) -> ObjId,
) {
    let seats = game.players.len();
    let mut kept = vec![false; seats];
    let mut taken = vec![0usize; seats];
    let size_after = |taken: usize| 7usize.saturating_sub(taken.saturating_sub(free));
    let order: Vec<Seat> = (0..seats).map(|k| (first + k) % seats).collect();
    while kept.iter().any(|k| !k) {
        let mut going = Vec::new();
        for &seat in &order {
            if kept[seat] {
                continue;
            }
            if size_after(taken[seat]) > 0 && wants(game, seat, taken[seat].saturating_sub(free)) {
                going.push(seat);
            } else {
                kept[seat] = true;
            }
        }
        for seat in going {
            taken[seat] += 1;
            redraw(game, seat);
            while game.player(seat).hand.len() > size_after(taken[seat]) {
                let card = bottom(game, seat);
                put_on_bottom(game, seat, card);
            }
        }
    }
}

/// Shuffles the hand into the library and draws seven
pub fn redraw(game: &mut Game, seat: Seat) {
    let hand = std::mem::take(&mut game.player_mut(seat).hand);
    for id in &hand {
        game.obj_mut(*id).zone = Zone::Library;
    }
    let mut library = std::mem::take(&mut game.player_mut(seat).library);
    library.extend(hand);
    game.rng.shuffle(&mut library);
    game.player_mut(seat).library = library;
    rules::draw(game, seat, 7);
}

/// A card from the hand to the bottom of the library
pub fn put_on_bottom(game: &mut Game, seat: Seat, card: ObjId) {
    let p = game.player_mut(seat);
    p.hand.retain(|x| *x != card);
    p.library.push(card);
    game.obj_mut(card).zone = Zone::Library;
}

/// Whether the AI with `config` (`None`: the quick answers alone) mulligans `seat`'s hand
pub fn wants(game: &Game, seat: Seat, taken: usize, config: Option<&crate::search::Config>) -> bool {
    let _ = config;
    rule_wants(game, seat, taken)
}

/// The card the AI with `config` puts on the bottom
pub fn bottom(game: &Game, seat: Seat, config: Option<&crate::search::Config>) -> ObjId {
    let _ = config;
    rule_bottom(game, seat)
}
