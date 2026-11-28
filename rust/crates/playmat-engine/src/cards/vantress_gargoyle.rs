//! Vantress Gargoyle: "Flying. Vantress Gargoyle can't attack unless defending player has seven or more cards in
//! their graveyard. Vantress Gargoyle can't block unless you have four or more cards in hand. {T}: Each player mills a
//! card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::restriction("VantressGargoyle.VantressGargoyleAttackEffect", this, &[Restrict::CantAttack(no_attack)]),
    Handler::restriction("VantressGargoyle.VantressGargoyleBlockEffect", this_few_cards, &[Restrict::CantBlock(no_block)]),
];

fn this(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// `canAttack(attacker, defender)`: no opponent it could attack (a player, or a planeswalker's controller) has
/// seven or more cards in their graveyard
fn no_attack(game: &Game, cx: &StaticCx, _: ObjId) -> bool {
    !(0..game.seats()).any(|p| cx.opponent(p) && !game.player(p).lost && game.player(p).graveyard.len() > 6)
}

/// `applies`: this, while you have fewer than four cards in hand
fn this_few_cards(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && game.player(cx.controller).hand.len() < 4
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
