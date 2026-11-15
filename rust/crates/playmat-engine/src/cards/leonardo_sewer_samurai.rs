//! Leonardo, Sewer Samurai: "Sneak {2}{W}{W}. Double strike. During your turn, you may cast creature spells with power
//! or toughness 1 or less from your graveyard. If you cast a spell this way, that creature enters with a finality
//! counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("LeonardoSewerSamurai.LeonardoSewerSamuraiEffect", AsThough::PlayThen(play, cast))];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`, during your turn: a creature card you own in your graveyard with power or toughness
/// 1 or less, cast as usual (no other mana or costs)
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    (seat == cx.controller
        && game.active == Some(seat)
        && o.owner == seat
        && o.zone == Zone::Graveyard
        && game.has_type(id, CardType::Creature)
        && (face.power <= 1 || face.toughness <= 1))
        .then(PlayWay::default)
}

/// `LeonardoSewerSamuraiWatcher`: the spell cast this way enters with a finality counter
fn cast(game: &mut Game, d: &mut dyn Decider, _source: ObjId, controller: Seat, spell: ObjId) -> Option<()> {
    super::add_counter_entering_creature_effect::add_for_spell(game, d, controller, spell, "finality 1")
}
