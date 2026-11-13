//! Ultron, Artificial Malevolence: "Whenever another nontoken artifact you control enters, you may pay {2}. If you
//! do, create a token that's a copy of it. If the token isn't a creature, it becomes a 2/2 Robot Villain creature in
//! addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("UltronArtificialMalevolence.UltronArtificialMaleveolenceEffect", apply)];

/// the artifact the trigger points at (or as it last was); a noncreature one copied as a 2/2 Robot Villain creature
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.target(0).and_then(|t| if let Target::Obj(o) = t { Some(o) } else { None }).or(cx.item.entering)
    else {
        return Some(());
    };
    let creature = game.has_type(of, CardType::Creature);
    let mods = |def: &mut CardDef| {
        if creature {
            return;
        }
        if !def.types.contains(&CardType::Creature) {
            def.types.push(CardType::Creature);
        }
        def.remove_pt_cda();
        def.power = 2;
        def.toughness = 2;
        for s in ["Robot", "Villain"] {
            if !def.subtypes.iter().any(|x| x == s) {
                def.subtypes.push(s.into());
            }
        }
    };
    token_copy_modified(game, d, of, you, 1, &mods);
    if d.stopped() {
        return None;
    }
    Some(())
}
