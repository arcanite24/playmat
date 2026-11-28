//! Ajani, Strength of the Pride: "+1: You gain life equal to the number of creatures you control plus the number of
//! planeswalkers you control." and "0: If you have at least 15 life more than your starting life total, exile Ajani,
//! Strength of the Pride and each artifact and creature your opponents control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("AjaniStrengthOfThePride.AjaniStrengthOfThePrideValue", creatures_and_planeswalkers),
    Handler::effect("AjaniStrengthOfThePride.AjaniStrengthOfThePrideEffect", apply),
];

/// creatures you control plus planeswalkers you control (a creature planeswalker counts twice)
fn creatures_and_planeswalkers(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mine = |t: CardType| {
        game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_type(**o, t)).count() as i32
    };
    mine(CardType::Creature) + mine(CardType::Planeswalker)
}

/// With 15 life more than the starting life: `ExileSourceEffect`, then `ExileAllEffect` of your opponents'
/// artifacts and creatures
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if game.player(you).life < game.starting_life + 15 {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        game.move_to(cx.source_id(), Zone::Exile);
    }
    let filter = Filter(vec![
        Pred::Controller { who: "opponent".into() },
        Pred::Or { xs: vec![Pred::Type { t: CardType::Artifact }, Pred::Type { t: CardType::Creature }] },
    ]);
    cx.run(game, d, &[EffectDef::ExileAllEffect { filter }]);
    Some(())
}
