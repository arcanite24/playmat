//! Soul-Shackled Zombie: "When this creature enters, exile up to two target cards from a single graveyard. If at
//! least one creature card was exiled this way, each opponent loses 2 life and you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoulShackledZombie.SoulShackledZombieEffect", apply)];

/// Every target card is exiled together; then, with a creature card among them, you gain 2 life and each opponent
/// loses 2 (Java's order: the gain first)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = (0..cx.legal.len()).flat_map(|i| cx.objs(i)).collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    if cards.iter().any(|c| game.has_type(*c, CardType::Creature)) {
        gain_life(game, you, 2);
        for i in 1..game.seats() {
            let opp = (you + i) % game.seats();
            lose_life(game, opp, 2);
        }
    }
    Some(())
}
