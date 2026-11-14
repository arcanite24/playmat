//! Nightmare Shepherd: "Whenever another nontoken creature you control dies, you may exile it. If you do, create a
//! token that's a copy of that creature, except it's 1/1 and it's a Nightmare in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NightmareShepherd.NightmareShepherdEffect", apply)];

/// The card that died (the trigger's pointer, still in the graveyard) exiled; then the token copy of the creature as
/// it last was (`setSavedPermanent(creatureDied)`), 1/1 and a Nightmare
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    token_copy_modified(game, d, died, you, 1, &|def: &mut CardDef| {
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
        let creature = def.types.contains(&CardType::Creature) || def.types.contains(&CardType::Kindred);
        if creature && !def.subtypes.iter().any(|s| s == "Nightmare") {
            def.subtypes.push("Nightmare".into());
        }
    });
    if d.stopped() {
        return None;
    }
    Some(())
}
