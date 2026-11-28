//! Steel Hellkite: "{X}: Destroy each nonland permanent with mana value X whose controller was dealt combat damage
//! by Steel Hellkite this turn. Activate only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SteelHellkite.SteelHellkiteDestroyEffect", apply)];

/// `SteelHellkiteWatcher`: the players this source dealt combat damage to this turn
/// (`Game::combat_damaged_players`); then `DestroyAllEffect` over the nonland permanents they control with mana
/// value X, in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let damaged: Vec<Seat> =
        game.combat_damaged_players.iter().filter(|(s, _)| *s == src).map(|(_, p)| *p).collect();
    if damaged.is_empty() {
        return Some(());
    }
    let x = cx.item.x;
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let ob = game.obj(*o);
            !game.has_type(*o, CardType::Land)
                && damaged.contains(&ob.controller)
                && ob.def.mana_value() as i32 == x
        })
        .collect();
    for o in doomed {
        destroy(game, o);
    }
    Some(())
}
