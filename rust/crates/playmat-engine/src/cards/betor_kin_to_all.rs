//! Betor, Kin to All: "At the beginning of your end step, if creatures you control have total toughness 10 or
//! greater, draw a card. Then if creatures you control have total toughness 20 or greater, untap each creature you
//! control. Then if creatures you control have total toughness 40 or greater, each opponent loses half their life,
//! rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("BetorKinToAll.BetorKinToAllCondition.TEN", ten),
    Handler::condition("BetorKinToAll.BetorKinToAllCondition.TWENTY", twenty),
    Handler::condition("BetorKinToAll.BetorKinToAllCondition.FORTY", forty),
    Handler::effect("BetorKinToAll.BetorKinToAllEffect", halve_opponents),
];

/// `ControlledCreaturesToughnessValue`: the total toughness of the creatures you control
fn total_toughness(game: &Game, you: Seat) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).1)
        .sum()
}

fn ten(game: &Game, you: Seat, _source: ObjId) -> bool {
    total_toughness(game, you) >= 10
}

fn twenty(game: &Game, you: Seat, _source: ObjId) -> bool {
    total_toughness(game, you) >= 20
}

fn forty(game: &Game, you: Seat, _source: ObjId) -> bool {
    total_toughness(game, you) >= 40
}

/// Each opponent, in turn order, loses half their life rounded up (`life / 2 + life % 2`)
fn halve_opponents(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    for p in opponents {
        let life = game.player(p).life;
        let amount = life / 2 + life % 2;
        if amount != 0 {
            lose_life(game, p, amount);
        }
    }
    Some(())
}
