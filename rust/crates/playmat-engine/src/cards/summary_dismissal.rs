//! Summary Dismissal: "Exile all other spells and counter all abilities."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SummaryDismissal.SummaryDismissalEffect", apply)];

/// Every stack object whose source isn't this spell, from the top (Java's stack iterates from its top): a spell is
/// exiled (`Spell.moveToExile`, not countered), an ability countered (it leaves the stack)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    let items: Vec<(ObjId, bool)> = game
        .stack
        .iter()
        .rev()
        .filter(|i| i.card != me)
        .map(|i| (i.card, i.ability.is_none() && i.delayed.is_none()))
        .collect();
    for (card, spell) in items {
        if spell {
            if game.obj(card).zone != Zone::Stack {
                continue;
            }
            Arc::make_mut(&mut game.stack).retain(|i| i.card != card || i.ability.is_some() || i.delayed.is_some());
            game.move_to(card, Zone::Exile);
        } else if let Some(k) =
            game.stack.iter().rposition(|i| i.card == card && (i.ability.is_some() || i.delayed.is_some()))
        {
            Arc::make_mut(&mut game.stack).remove(k);
        }
    }
    Some(())
}
