//! Calix, Guided by Fate: "Constellation — Whenever Calix or another enchantment you control enters, put a +1/+1
//! counter on target creature. Whenever Calix or an enchanted creature you control deals combat damage to a player,
//! you may create a token that's a copy of a nonlegendary enchantment you control. Do this only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("CalixGuidedByFate.CalixGuidedByFatePredicate", calix_or_enchanted),
    Handler::effect("CalixGuidedByFate.CalixGuidedByFateEffect", apply),
];

/// Calix itself, or another permanent its controller controls with an Aura attached
fn calix_or_enchanted(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    if source == Some(id) {
        return true;
    }
    game.obj(id).controller == you
        && game.obj(id).zone == Zone::Battlefield
        && game.filter_matches(&Filter(vec![Pred::Enchanted]), you, source, id)
}

/// Up to one nonlegendary enchantment you control (`TargetPermanent(0, 1, filter, true)` through `choose`), then a
/// token copy of it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let options: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you
                && game.has_type(*o, CardType::Enchantment)
                && !game.obj(*o).def.supertypes.iter().any(|s| s == "LEGENDARY")
        })
        .collect();
    if options.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &options, &options, 0, 1, "nonlegendary enchantment you control")?;
    let Some(&of) = chosen.first() else { return Some(()) };
    token_copy(game, d, of, you, None);
    (!d.stopped()).then_some(())
}
