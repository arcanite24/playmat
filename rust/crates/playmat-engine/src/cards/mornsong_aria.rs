//! Mornsong Aria: "Players can't draw cards or gain life. At the beginning of each player's draw step, that player
//! loses 3 life, searches their library for a card, puts it into their hand, then shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "MornsongAria.MornsongAriaStaticEffect",
        &[EventType::DrawCard, EventType::GainLife],
        always,
        stop,
    ),
    Handler::effect("MornsongAria.MornsongAriaTriggerEffect", apply)
        .effects(&["new SearchLibraryPutInHandEffect(new TargetCardInLibrary(), false)"]),
];

/// `DRAW_CARD` and `GAIN_LIFE`, anyone's (`ContinuousRuleModifyingEffectImpl`: the event doesn't happen)
fn always(_: &Game, _: &StaticCx, _: &Event) -> bool {
    true
}

fn stop(_: &mut Game, _: &StaticCx, _: &mut Event) -> bool {
    true
}

/// The active player loses 3 life, then searches their library for a card, puts it into their hand and shuffles
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = game.active else { return Some(()) };
    lose_life_by(game, p, 3, Some(cx.source_id()), false);
    cx.run_as(game, d, cx.effects, p);
    (!d.stopped()).then_some(())
}
