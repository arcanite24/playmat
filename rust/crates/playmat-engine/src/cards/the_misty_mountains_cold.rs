//! The Misty Mountains Cold: "Create a Treasure token. Then if you control four or more Treasures, sacrifice this
//! Saga. If you do, create a 6/6 red Dragon creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheMistyMountainsCold.TheMistyMountainsColdEffect", apply)
    .tokens(&["TreasureToken", "Dragon66Token"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    let treasures = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_subtype(**b, "Treasure"))
        .count();
    if treasures < 4 || !cx.source_on_battlefield(game) {
        return Some(());
    }
    sacrifice(game, cx.source_id());
    if game.obj(cx.source_id()).zone != Zone::Battlefield {
        cx.create_tokens(game, d, &cx.tokens[1], 1);
    }
    Some(())
}
