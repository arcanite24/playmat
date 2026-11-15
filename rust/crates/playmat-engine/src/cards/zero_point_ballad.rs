//! Zero Point Ballad: "Destroy all creatures with toughness X or less. You lose X life. If X is 6 or more, return a
//! creature card put into a graveyard this way to the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZeroPointBallad.ZeroPointBalladEffect", apply)];

/// Each creature with toughness X or less destroyed in battlefield order (those then in a graveyard remembered),
/// you lose X life, then with X of 6 or more you choose one of those still a creature card in a graveyard
/// (`TargetCardInGraveyard`, not targeted) and it enters under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && game.power_toughness(*o).1 <= x)
        .collect();
    let mut died = Vec::new();
    for o in doomed {
        destroy(game, o);
        if game.obj(o).zone == Zone::Graveyard {
            died.push(o);
        }
    }
    lose_life(game, you, x);
    if x < 6 {
        return Some(());
    }
    let options: Vec<ObjId> = died
        .into_iter()
        .filter(|o| {
            game.obj(*o).zone == Zone::Graveyard && !game.obj(*o).token && game.obj(*o).is(CardType::Creature)
        })
        .collect();
    if options.is_empty() {
        return Some(());
    }
    let rule = format!("creature card put into a graveyard with {}", log::object(game, cx.source_id()));
    let chosen = choose_cards(game, d, you, &options, &options, 1, 1, &rule)?;
    let Some(&c) = chosen.first() else { return Some(()) };
    put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield);
    (!d.stopped()).then_some(())
}
