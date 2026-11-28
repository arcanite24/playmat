//! Winter, Misanthropic Guide: "Delirium — As long as there are four or more card types among cards in your
//! graveyard, each opponent's maximum hand size is equal to seven minus the number of those card types."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("WinterMisanthropicGuide.WinterMisanthropicGuideEffect", Rule::MaxHandSize(opponents))];

/// `CardTypesInGraveyardCount.YOU`: four or more, and each opponent's `setMaxHandSize(max(7 - count, 0))`
fn opponents(game: &Game, cx: &StaticCx, seat: Seat) -> Option<usize> {
    if !cx.opponent(seat) {
        return None;
    }
    let n = game.value(&crate::card::Amount::Of(crate::card::Value::CardTypesInYourGraveyard), cx.controller, cx.source, 0);
    (n >= 4).then(|| (7 - n).max(0) as usize)
}
