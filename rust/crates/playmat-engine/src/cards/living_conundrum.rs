//! Living Conundrum: "Hexproof. If you would draw a card while your library has no cards in it, skip that draw
//! instead. As long as there are no cards in your library, Living Conundrum has base power and toughness 10/10 and
//! has flying and vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement("LivingConundrum.LivingConundrumDrawEffect", &[EventType::DrawCard], applies, skip),
    Handler::condition("LivingConundrum.LivingConundrumCondition", empty_library),
];

/// your draw while your library is empty
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller && game.player(ev.player).library.is_empty()
}

/// the draw is skipped
fn skip(_: &mut Game, _: &StaticCx, _: &mut Event) -> bool {
    true
}

fn empty_library(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).library.is_empty()
}
