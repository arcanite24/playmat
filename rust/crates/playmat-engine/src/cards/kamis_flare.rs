//! Kami's Flare: "Kami's Flare deals 3 damage to target creature or planeswalker. Kami's Flare also deals 2 damage to
//! that permanent's controller if you control a modified creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KamisFlare.KamisFlareEffect", apply)];

/// the modified creature is looked for after the 3 damage (it may have died of it)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    deal_damage(game, cx.source_id(), Target::Obj(o), 3, false);
    let you = cx.controller();
    let modified = Filter(vec![Pred::Modified, Pred::Type { t: CardType::Creature }]);
    let any = game
        .battlefield
        .iter()
        .any(|p| game.obj(*p).controller == you && game.filter_matches(&modified, you, Some(cx.source_id()), *p));
    if any {
        deal_damage(game, cx.source_id(), Target::Player(game.obj(o).controller), 2, false);
    }
    Some(())
}
