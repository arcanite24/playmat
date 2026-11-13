//! Electrodominance: "Electrodominance deals X damage to any target. You may cast a spell with mana value X or less
//! from your hand without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Electrodominance.ElectrodominanceEffect", apply)];

/// `CardUtil.castSpellWithAttributesForFree` over your hand with a "mana value X or less" filter: one castable card
/// is taken as it is, more are chosen among (up to one); then `chooseUse` and the free cast
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let options: Vec<ObjId> = game
        .player(you)
        .hand
        .iter()
        .copied()
        .filter(|&c| !game.has_type(c, CardType::Land) && game.obj(c).def.mana_value() as i32 <= x)
        .collect();
    let card = match options.len() {
        0 => return Some(()),
        1 => options[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    super::breaching_dragonstorm::cast_for_free(game, d, you, card, |_, _| true)?;
    Some(())
}
