//! Feldon, Ronom Excavator: "Whenever Feldon is dealt damage, exile that many cards from the top of your library.
//! Choose one of them. Until the end of your next turn, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FeldonRonomExcavator.FeldonRonomExcavatorEffect", apply)];

/// The batch's damage (the trigger's X) many cards to exile in one move; one of those still there is chosen (a
/// lone card without asking), playable through the end of your next turn (`makeCardPlayable`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let damage = cx.item.x;
    if damage < 1 {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(damage as usize).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
    }
    let cards: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let card = match cards.len() {
        0 => return Some(()),
        1 => cards[0],
        _ => {
            let rule = item_rule(game, cx.item);
            *choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?.first()?
        }
    };
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    let o = game.obj_mut(card);
    o.may_play = Some((you, until));
    o.may_play_end_step = false;
    Some(())
}
