//! `AscendEffect`: ascend on an instant or sorcery ("If you control ten or more permanents, you get the city's
//! blessing for the rest of the game.": Golden Demise, Secrets of the Golden City, Vona's Hunger)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AscendEffect", apply)];

/// `AscendAbility.checkAscend(game, source, true)`, with its log lines
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = log::player(you);
    if game.player(you).citys_blessing {
        log::inform(game, format!("{p} already has the city's blessing."));
        return Some(());
    }
    let mine = game.battlefield.iter().filter(|o| game.obj(**o).controller == you).count();
    if mine > 9 {
        game.player_mut(you).citys_blessing = true;
        log::inform(game, format!("{p} gets the city's blessing for the rest of the game."));
    } else {
        log::inform(game, format!("{p} does not get the city's blessing."));
    }
    Some(())
}
