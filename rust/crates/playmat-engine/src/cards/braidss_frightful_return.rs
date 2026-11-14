//! Braids's Frightful Return, chapter II: "Target opponent may sacrifice a nonland, nontoken permanent. If they
//! don't, they lose 2 life and you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BraidssFrightfulReturn.BraidssFrightfulReturnEffect", apply)];

/// Asked only with such a permanent; `chooseTarget` of one, and the sacrifice must happen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.obj(*o).token && !game.has_type(*o, CardType::Land))
        .collect();
    let mut sacrificed = false;
    if !theirs.is_empty() && choose_use(game, d, p, "Sacrifice a nonland, nontoken permanent?")? {
        let rule = item_rule(game, cx.item);
        if let Some(&o) = choose_cards(game, d, p, &theirs, &theirs, 1, 1, &rule)?.first() {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
                sacrificed = game.obj(o).zone != Zone::Battlefield;
            }
        }
    }
    if !sacrificed {
        lose_life(game, p, 2);
        draw(game, cx.controller(), 1);
    }
    Some(())
}
