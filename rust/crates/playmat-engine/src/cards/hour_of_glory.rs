//! Hour of Glory: "Exile target creature. If that creature was a God, its controller reveals their hand and exiles
//! all cards with the same name as that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HourOfGlory.HourOfGloryEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let god = game.has_subtype(o, "God");
    let (p, name) = (game.obj(o).controller, game.obj(o).name().to_string());
    game.move_to(o, Zone::Exile);
    if god {
        let same: Vec<ObjId> = game.player(p).hand.iter().copied().filter(|c| game.obj(*c).name() == name).collect();
        for c in same {
            game.move_to(c, Zone::Exile);
        }
    }
    Some(())
}
