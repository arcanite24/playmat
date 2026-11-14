//! Selfless Glyphweaver // Deadly Vanity: "Choose a creature or planeswalker, then destroy all other creatures and
//! planeswalkers."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SelflessGlyphweaver.DeadlyVanityEffect", apply)];

/// `choose(TargetCreatureOrPlaneswalker().withNotTarget(true))` (none on the battlefield: nothing asked), then
/// `DestroyAllEffect` of the other creatures and planeswalkers
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let both = |g: &Game, o: ObjId| g.has_type(o, CardType::Creature) || g.has_type(o, CardType::Planeswalker);
    let possible: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| both(game, *o)).collect();
    let mut kept = None;
    if !possible.is_empty() {
        let rule = item_rule(game, cx.item);
        kept = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first().copied();
    }
    let doomed: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| Some(*o) != kept && both(game, *o)).collect();
    for o in doomed {
        destroy(game, o);
    }
    Some(())
}
