//! Arid Archway: "When this land enters, return a land you control to its owner's hand. If another Desert was
//! returned this way, surveil 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AridArchway.AridArchwayEffect", apply)];

/// `choose(TargetPermanent(land you control), notTarget)`; a Desert other than this land returned means surveil 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Land))
        .collect();
    if lands.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(land) = choose_cards(game, d, you, &lands, &lands, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let (src, zcc) = cx.source;
    let other_desert = game.has_subtype(land, "Desert") && !(land == src && game.obj(land).zcc == zcc);
    game.move_to(land, Zone::Hand);
    if other_desert {
        cx.run(game, d, &[EffectDef::SurveilEffect { amount: crate::card::Amount::Fixed(1) }]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
