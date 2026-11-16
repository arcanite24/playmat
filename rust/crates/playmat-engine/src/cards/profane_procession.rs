//! Profane Procession // Tomb of the Dusk Rose: "{3}{W}{B}: Exile target creature. Then if there are three or more
//! cards exiled with Profane Procession, transform it." // "{T}: Add one mana of any color. {2}{W}{B}, {T}: Put a
//! creature card exiled with this permanent onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ProfaneProcession.ProfaneProcessionCondition", three_exiled),
    Handler::effect("ProfaneProcession.TombOfTheDuskRoseEffect", tomb),
];

/// the cards in the source's exile zone (`CardUtil.getExileZoneId`, its current zone change count)
fn exiled(game: &Game, source: ObjId) -> Vec<ObjId> {
    let zcc = game.obj(source).zcc;
    game.exiled_for_source
        .iter()
        .filter(|(s, z, c)| *s == source && *z == zcc && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c)
        .collect()
}

/// the source's exile zone holds three or more cards
fn three_exiled(game: &Game, _: Seat, source: ObjId) -> bool {
    exiled(game, source).len() >= 3
}

/// a creature card of the zone (`TargetCardInExile`, not targeted) chosen with `Player.choose`, onto the battlefield
/// under your control
fn tomb(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut creatures: Vec<ObjId> =
        exiled(game, cx.source_id()).into_iter().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    creatures.dedup();
    if creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &creatures, &creatures, 1, 1, &rule)?.first() else {
        return Some(());
    };
    put_onto_battlefield(game, d, card, you);
    (!d.stopped()).then_some(())
}
