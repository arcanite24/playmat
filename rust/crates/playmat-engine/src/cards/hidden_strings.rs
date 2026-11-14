//! Hidden Strings: "You may tap or untap target permanent, then you may tap or untap another target permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HiddenStrings.HiddenStringsEffect", hidden_strings)];

/// each target still on the battlefield in turn: "Untap <name>?" when tapped, else "Tap <name>?"
fn hidden_strings(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let tapped = game.obj(o).tapped;
        let name = game.obj(o).def.name.clone();
        let message = if tapped { format!("Untap {name}?") } else { format!("Tap {name}?") };
        if choose_use(game, d, you, &message)? {
            if tapped {
                untap_permanent(game, o);
            } else {
                tap(game, o);
            }
        }
    }
    Some(())
}
