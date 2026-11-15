//! Glen Elendra's Answer: "This spell can't be countered. Counter all spells your opponents control and all
//! abilities your opponents control. Create a 1/1 blue and black Faerie creature token with flying for each spell and
//! ability countered this way."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GlenElendrasAnswer.GlenElendrasAnswerEffect", apply).tokens(&["FaerieBlueBlackToken"])];

/// each stack object an opponent controls, from the top down (`getStack().stream()`), countered (`Stack.counter`:
/// an ability leaves the stack, a spell that can be countered goes to its owner's graveyard); a Faerie for each
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut count = 0;
    let objects: Vec<(ObjId, bool)> = game
        .stack
        .iter()
        .rev()
        .filter(|i| i.controller != you)
        .map(|i| (i.card, i.ability.is_some() || i.delayed.is_some()))
        .collect();
    for (o, ability) in objects {
        if ability {
            if let Some(k) =
                game.stack.iter().rposition(|i| i.card == o && i.controller != you && (i.ability.is_some() || i.delayed.is_some()))
            {
                Arc::make_mut(&mut game.stack).remove(k);
                count += 1;
            }
        } else if game.obj(o).zone == Zone::Stack && !cant_be_countered(game, o) {
            note_countered(game, o, cx.source_id());
            Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some());
            game.move_to(o, Zone::Graveyard);
            count += 1;
        }
    }
    if count > 0 {
        cx.create_tokens(game, d, &cx.tokens[0], count);
    }
    Some(())
}
