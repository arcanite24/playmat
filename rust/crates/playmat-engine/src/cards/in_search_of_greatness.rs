//! In Search of Greatness: "At the beginning of your upkeep, you may cast a permanent spell from your hand with mana
//! value equal to 1 plus the highest mana value among other permanents you control without paying its mana cost. If
//! you don't, scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InSearchOfGreatness.InSearchOfGreatnessEffect", apply)];

/// `GreatestAmongPermanentsValue.MANAVALUE_OTHER_CONTROLLED_PERMANENTS` plus 1, then
/// `CardUtil.castSpellWithAttributesForFree` over your hand with a permanent card of that mana value (one is taken
/// as it is, more are chosen among, up to one; then `chooseUse` and the free cast); not cast: scry 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let greatest = game
        .battlefield
        .iter()
        .filter(|o| **o != me && game.obj(**o).controller == you)
        .map(|o| game.obj(*o).def.mana_value() as i32)
        .max()
        .unwrap_or(0);
    let mv = greatest + 1;
    let options: Vec<ObjId> = game
        .player(you)
        .hand
        .iter()
        .copied()
        .filter(|&c| {
            game.obj(c).def.is_permanent_card()
                && !game.has_type(c, CardType::Land)
                && game.obj(c).def.mana_value() as i32 == mv
        })
        .collect();
    let card = match options.len() {
        0 => None,
        1 => Some(options[0]),
        _ => {
            let rule = item_rule(game, cx.item);
            choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first().copied()
        }
    };
    let cast = match card {
        Some(c) => super::breaching_dragonstorm::cast_for_free(game, d, you, c, |_, _| true)?,
        None => false,
    };
    if !cast {
        let rule = item_rule(game, cx.item);
        if !scry(game, d, you, 1, &rule) {
            return None;
        }
    }
    Some(())
}
