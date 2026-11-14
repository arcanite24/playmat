//! Cavalcade of Calamity: "Whenever a creature you control with power 1 or less attacks, this enchantment deals 1
//! damage to the player or planeswalker that creature is attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CavalcadeOfCalamity.CavalcadeOfCalamityEffect", apply)];

/// `game.getCombat().getDefenderId(attacker)`: the planeswalker it attacks, else the player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let a = cx.obj(0)?;
    let o = game.obj(a);
    let to = match (o.attacking_pw, o.attacking) {
        (Some((pw, zcc)), _) if game.obj(pw).zone == Zone::Battlefield && game.obj(pw).zcc == zcc => Target::Obj(pw),
        (Some(_), _) => return Some(()),
        (None, Some(p)) => Target::Player(p),
        (None, None) => return Some(()),
    };
    deal_damage(game, cx.source_id(), to, 1, false);
    Some(())
}
