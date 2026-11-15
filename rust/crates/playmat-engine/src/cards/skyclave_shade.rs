//! Skyclave Shade: "Kicker {2}{B}. Skyclave Shade can't block. If Skyclave Shade was kicked, it enters with two
//! +1/+1 counters on it. Landfall — Whenever a land you control enters, if Skyclave Shade is in your graveyard and
//! it's your turn, you may cast it from your graveyard this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("SkyclaveShade.SkyclaveShadeCondition", in_graveyard_your_turn),
    Handler::as_though("SkyclaveShade.SkyclaveShadeEffect", AsThough::Play(play)).on_start(this_card),
];

/// it's your turn and this card is in a graveyard
fn in_graveyard_your_turn(game: &Game, you: Seat, source: ObjId) -> bool {
    game.active == Some(you) && game.obj(source).zone == Zone::Graveyard
}

/// the card as it is while the effect starts (`getStackMomentSourceZCC`)
fn this_card(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    Some(vec![(Target::Obj(cx.source), game.obj(cx.source).zcc)])
}

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, by the ability's controller, still in the graveyard it was in
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let same = cx.target_obj(game, 0) == Some(id);
    (same && id == cx.source && seat == cx.controller && game.obj(id).zone == Zone::Graveyard).then(PlayWay::default)
}
