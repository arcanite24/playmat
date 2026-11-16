//! Revealing Wind: "Prevent all combat damage that would be dealt this turn. You may look at each face-down creature
//! that's attacking or blocking."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RevealingWind.RevealingWindEffect", apply).filters(&["filter"])];

/// While there's a face-down attacking or blocking creature: "Look at a face-down attacking creature?", then
/// `chooseTarget` of one (not targeted) and `lookAtCards` of it, titled by the source
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    loop {
        let options = matching(game, &cx.filters[0], you, src);
        if options.is_empty() || !choose_use(game, d, you, "Look at a face-down attacking creature?")? {
            break;
        }
        let rule = item_rule(game, cx.item);
        let Some(&card) = choose_cards(game, d, you, &options, &options, 1, 1, &rule)?.first() else { continue };
        let zcc = game.obj(src).zcc;
        game.looked_at.push(crate::state::LookedAt { viewer: you, source: src, zcc, bare: false, cards: vec![card] });
        if log::kept(game) {
            let line = format!("{} look at a face-down attacking creature", log::player(you));
            log::inform(game, line);
        }
    }
    Some(())
}
