//! Soul of Shandalar: "First strike. {3}{R}{R}: Soul of Shandalar deals 3 damage to target player or planeswalker
//! and 3 damage to up to one target creature that player or that planeswalker's controller controls. {3}{R}{R},
//! Exile Soul of Shandalar from your graveyard: Soul of Shandalar deals 3 damage to target player or planeswalker and
//! 3 damage to up to one target creature that player or that planeswalker's controller controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "SoulOfShandalar.SoulOfShandalarTarget",
        "new TargetPermanent(0, 1, new FilterCreaturePermanent(\"creature that the targeted player or planeswalker's controller controls\"))",
        controlled_by_first,
    ),
    Handler::effect("SoulOfShandalar.SoulOfShandalarEffect", apply),
];

/// a creature the first target (a player, or a planeswalker's controller) controls, not that planeswalker itself;
/// with no first target, any
fn controlled_by_first(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Player(p)) => game.obj(id).controller == *p,
        Some(Target::Obj(pw)) => *pw != id && game.obj(id).controller == game.obj(*pw).controller,
        None => true,
    }
}

/// 3 damage to the player or planeswalker, then 3 to the creature (each if it's still a legal target)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if let Some(t) = cx.target(0) {
        deal_damage(game, src, t, 3, false);
    }
    if let Some(c) = cx.obj(1) {
        if game.obj(c).zone == Zone::Battlefield {
            deal_damage(game, src, Target::Obj(c), 3, false);
        }
    }
    Some(())
}
