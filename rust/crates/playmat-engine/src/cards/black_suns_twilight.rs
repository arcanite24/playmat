//! Black Sun's Twilight: "Up to one target creature gets -X/-X until end of turn. If X is 5 or more, return a
//! creature card with mana value X or less from your graveyard to the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlackSunsTwilight.BlackSunsTwilightEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURE"])];

/// X below 5 or no creature card of mana value X or less in your graveyard: nothing. Else `choose` one (not
/// targeted) and `moveCards` it onto the battlefield tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 5 {
        return Some(());
    }
    let you = cx.controller();
    let graveyard = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = graveyard
        .iter()
        .copied()
        .filter(|c| {
            game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)
                && (game.obj(*c).def.mana_value() as i32) < x + 1
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &graveyard, &possible, 1, 1, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
