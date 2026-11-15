//! Angel of Destiny: "Whenever a creature you control deals combat damage to a player, you and that player each gain
//! that much life. At the beginning of your end step, if you have at least 15 life more than your starting life
//! total, each player Angel of Destiny attacked this turn loses the game."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AngelOfDestiny.AngelOfDestinyGainLifeEffect", gain),
    Handler::effect("AngelOfDestiny.AngelOfDestinyLoseEffect", lose),
];

/// "that much" is the damage (the trigger's X); you gain it, then the damaged player (the pointer) does
fn gain(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let damage = cx.item.x;
    gain_life(game, cx.controller(), damage);
    if let Some(p) = cx.player(0) {
        gain_life(game, p, damage);
    }
    Some(())
}

/// `AngelOfDestinyWatcher`: the players this Angel (as it was when the trigger went on the stack) was declared
/// attacking this turn; each one loses (`PlayerImpl.lost`: unless they can't lose)
fn lose(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (angel, zcc) = cx.source;
    let mut players: Vec<Seat> = Vec::new();
    for (a, z, p) in &game.watch.attacked_players {
        if *a == angel && *z == zcc && !players.contains(p) {
            players.push(*p);
        }
    }
    for p in players {
        if game.player(p).lost || game.player(p).won || cant_lose_or_win(game, p, false) {
            continue;
        }
        game.player_mut(p).lost = true;
        if log::kept(game) {
            let line = format!("{} has lost the game", log::player(p));
            log::inform(game, line);
        }
    }
    Some(())
}
