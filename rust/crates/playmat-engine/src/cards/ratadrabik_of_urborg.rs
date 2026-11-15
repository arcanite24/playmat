//! Ratadrabik of Urborg: "Vigilance, ward {2}. Other Zombies you control have vigilance. Whenever another legendary
//! creature you control dies, create a token that's a copy of that creature, except it's not legendary and it's a 2/2
//! black Zombie in addition to its other colors and types."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::effect("RatadrabikOfUrborg.RatadrabikOfUrborgEffect", apply)];

/// the creature that died, as it last was (`getFirstTargetPermanentOrLKI`): `CreateTokenCopyTargetEffect` with
/// 2/2, a Zombie in addition, not legendary, and its colors plus black (`setOnlyColor(colors.union(B))`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let mut colors = game.obj(died).copiable().colors.clone();
    if !colors.contains(&Color::B) {
        colors.push(Color::B);
    }
    let colors: Vec<Color> = Color::ALL.iter().copied().filter(|c| colors.contains(c)).collect();
    token_copy_modified(game, d, died, you, 1, &|def: &mut CardDef| {
        def.supertypes.retain(|s| s != "LEGENDARY");
        def.remove_pt_cda();
        def.power = 2;
        def.toughness = 2;
        let creature = def.types.contains(&CardType::Creature) || def.types.contains(&CardType::Kindred);
        if creature && !def.subtypes.iter().any(|s| s == "Zombie") {
            def.subtypes.push("Zombie".into());
        }
        def.colors = colors.clone();
    });
    if d.stopped() {
        return None;
    }
    Some(())
}
