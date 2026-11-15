//! Noctis, Prince of Lucis: "Lifelink. You may cast artifact spells from your graveyard by paying 3 life in addition
//! to paying their other costs. If you cast a spell this way, that artifact enters with a finality counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("NoctisPrinceOfLucis.NoctisPrinceOfLucisEffect", AsThough::PlayThen(play, cast))];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: an artifact card you own in your graveyard, for its mana cost
/// (`setCastSourceIdWithAlternateMana`) and 3 life besides its other costs
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    (seat == cx.controller && o.owner == seat && o.zone == Zone::Graveyard && game.has_type(id, CardType::Artifact))
        .then(|| PlayWay { mana: Some(face.cost.clone()), costs: vec![CostDef::PayLifeCost { amount: 3 }] })
}

/// `NoctisPrinceOfLucisWatcher`: the spell cast this way enters with a finality counter
fn cast(game: &mut Game, d: &mut dyn Decider, _source: ObjId, controller: Seat, spell: ObjId) -> Option<()> {
    super::add_counter_entering_creature_effect::add_for_spell(game, d, controller, spell, "finality 1")
}
