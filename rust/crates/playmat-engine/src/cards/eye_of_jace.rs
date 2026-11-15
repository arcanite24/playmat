//! Eye of Jace: "At the beginning of your upkeep, surveil 1. Then if there are seven or more cards in your graveyard,
//! sacrifice this artifact, it deals 2 damage to each opponent, and you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("EyeOfJace.EyeOfJaceEffect", apply).effects(&["new SurveilEffect(1)"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    if game.player(you).graveyard.len() < 7 {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        sacrifice(game, cx.source_id());
    }
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    for p in opponents {
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    gain_life(game, you, 2);
    Some(())
}
