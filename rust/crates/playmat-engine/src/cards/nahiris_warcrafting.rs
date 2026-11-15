//! Nahiri's Warcrafting: "Nahiri's Warcrafting deals 5 damage to target creature, planeswalker, or battle. Look at the
//! top X cards of your library, where X is the excess damage dealt this way. You may exile one of those cards. Put
//! the rest on the bottom of your library in a random order. You may play the exiled card this turn."

use super::damage_with_excess_effect::damage_with_excess;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NahirisWarcrafting.NahirisWarcraftingEffect", apply)];

/// 5 damage with excess (`damageWithExcess`); with any, the top X cards (looked at, no window),
/// `player.choose(TargetCardInLibrary(0, 1))`, the one chosen exiled and playable this turn
/// (`makeCardPlayable(…, false, EndOfTurn, false)`), the rest to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let excess = damage_with_excess(game, cx.source_id(), o, 5);
    if excess < 1 {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(excess as usize).copied().collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &top, 0, 1, &rule)?;
    if let Some(&card) = chosen.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            let turn = game.turn;
            let o = game.obj_mut(card);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false).then_some(())
}
