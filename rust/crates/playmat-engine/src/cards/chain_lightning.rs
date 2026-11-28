//! Chain Lightning: "Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may
//! pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChainLightning.ChainLightningEffect", apply)];

/// 3 damage to the target; then the player dealt it (or the permanent's controller) is asked "Pay {R}{R} to copy the
/// spell?" and pays: a copy of this spell under them, with new targets if they choose (`createCopyOnStack`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.target(0) else { return Some(()) };
    let affected = match target {
        Target::Player(p) => p,
        Target::Obj(o) => {
            if game.obj(o).zone != Zone::Battlefield {
                return Some(());
            }
            game.obj(o).controller
        }
    };
    deal_damage(game, cx.source_id(), target, 3, false);
    if !choose_use(game, d, affected, "Pay {R}{R} to copy the spell?")? {
        return Some(());
    }
    let paid = pay_mana(game, d, affected, &ManaCost::parse("{R}{R}").ok()?);
    if d.stopped() {
        return None;
    }
    if paid && cx.item.ability.is_none() && cx.item.delayed.is_none() {
        copy_spell(game, d, cx.item, affected);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
