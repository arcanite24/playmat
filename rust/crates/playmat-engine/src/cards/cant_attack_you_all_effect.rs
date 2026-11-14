//! `CantAttackYouAllEffect(WhileOnBattlefield, filter, true)`: "Creatures with flying can't attack you or planeswalkers
//! you control." (Sandwurm Convergence). In a duel every defender an opponent's creature can attack is you or a
//! planeswalker of yours, so a creature of the filter an opponent controls can't attack at all.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("CantAttackYouAllEffect", applies, &[Restrict::CantAttack(no_attack)]).args(&[
        "while_on_battlefield",
        "filter",
        "one_of:true",
    ])];

fn applies(game: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    cx.opponent(game.obj(attacker).controller) && cx.matches(game, 0, attacker)
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}
