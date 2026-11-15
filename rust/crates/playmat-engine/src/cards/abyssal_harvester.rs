//! Abyssal Harvester: "{T}: Exile target creature card from a graveyard that was put there this turn. Create a token
//! that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens
//! you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AbyssalHarvester.AbyssalHarvesterEffect", apply)];

/// The card exiled (copied even when that move is replaced), a token copy of it that's also a Nightmare, then every
/// other Nightmare token you control exiled together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = cx.obj(0)?;
    game.move_to(card, Zone::Exile);
    let before: Vec<ObjId> = game.battlefield.iter().copied().collect();
    let copy: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CreateTokenCopyTargetEffect", "add_subtypes": ["Nightmare"]}))
            .ok()?;
    cx.run_on(game, d, &[copy], vec![Target::Obj(card)]);
    if d.stopped() {
        return None;
    }
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            before.contains(o)
                && game.obj(*o).token
                && game.obj(*o).controller == you
                && game.has_subtype(*o, "Nightmare")
        })
        .collect();
    for o in others {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Exile);
        }
    }
    Some(())
}
