//! You Find Some Prisoners: "• Interrogate Them — Exile the top three cards of target opponent's library. Choose one
//! of them. Until the end of your next turn, you may play that card, and you may spend mana as though it were mana
//! of any color to cast it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("YouFindSomePrisoners.YouFindSomePrisonersEffect", apply)];

/// the three exiled, then `player.choose` of one of them (`TargetCardInExile`, not a target) and
/// `CardUtil.makeCardPlayable(…, Duration.UntilEndOfYourNextTurn, true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = cx.player(0)?;
    let cards: Vec<ObjId> = game.player(opponent).library.iter().take(3).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let exiled: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if exiled.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &exiled, &exiled, 1, 1, &rule)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    let o = game.obj_mut(card);
    o.may_play = Some((you, until));
    o.may_play_end_step = false;
    o.may_play_while = None;
    o.any_mana = true;
    Some(())
}
