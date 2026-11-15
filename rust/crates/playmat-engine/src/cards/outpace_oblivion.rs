//! Outpace Oblivion: "{2}, Sacrifice this enchantment: It deals 2 damage to each player who doesn't have max speed."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OutpaceOblivion.OutpaceOblivionEffect", apply)];

/// each player in range, from its controller on, whose speed is less than 4
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost && game.player(p).speed < 4 {
            deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
        }
    }
    Some(())
}
