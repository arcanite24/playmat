//! Make Your Own Luck: "Look at the top three cards of your library. You may exile a nonland card from among them.
//! If you do, it becomes plotted. Put the rest into your hand."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("MakeYourOwnLuck.MakeYourOwnLuckEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])];

/// `LookLibraryAndPickControllerEffect(3, 1, nonland, EXILED, HAND)` (optional, one card), the picked card exiled and
/// plotted (`PlotAbility.doExileAndPlotCard`), then the rest into the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    let e = EffectDef::LookLibraryAndPickControllerEffect {
        look: 3,
        look_count: None,
        look_amount: None,
        pick: 1,
        filter: cx.filters[0].clone(),
        picked: PutCards::Exiled,
        looked: PutCards::Hand,
        up_to: false,
        optional: true,
        pick_x: false,
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let turn = game.turn;
    for c in top {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).plotted_turn = Some(turn);
            let fired = game.triggers([c], |_, src, _, t| src == c && matches!(t, Trigger::BecomesPlotted));
            game.pending.extend(fired);
        }
    }
    Some(())
}
