//! Combustible Gearhulk: "When this creature enters, target opponent may have you draw three cards. If the player
//! doesn't, you mill three cards, then this creature deals damage to that player equal to the total mana value of
//! those cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CombustibleGearhulk.CombustibleGearhulkEffect", apply)];

/// The opponent's `chooseUse`; a yes draws three (`DrawCardSourceControllerEffect(3)`), a no mills three and deals
/// their total mana value to that player (`CombustibleGearhulkMillAndDamageEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let message = format!("Have {} draw three cards?", log::player(you));
    if choose_use(game, d, opponent, &message)? {
        draw(game, you, 3);
        return Some(());
    }
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let total: i32 = top.iter().map(|c| game.obj(*c).def.mana_value() as i32).sum();
    deal_damage(game, cx.source_id(), Target::Player(opponent), total, false);
    Some(())
}
