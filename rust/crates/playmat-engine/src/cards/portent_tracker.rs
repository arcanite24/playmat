//! Portent Tracker: "{T}: Untap target land. {T}: Choose target battle. If an opponent protects it, remove a defense
//! counter from it. Otherwise, put a defense counter on it. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PortentTracker.PortentTrackerEffect", apply)];

/// the target battle (still there): protected by an opponent (`ProtectedByOpponentPredicate`), a defense counter
/// off it (the last one off: its Siege trigger, as damage's), else one on it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(battle) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    if game.obj(battle).protector.is_none_or(|p| p == you) {
        add_counters_by(game, battle, "defense", 1, Some(you));
        return Some(());
    }
    if game.obj(battle).counter("defense") > 0 {
        game.obj_mut(battle).add_counters("defense", -1);
        if game.obj(battle).counter("defense") == 0 {
            let fired = game.triggers([battle], |_, _, _, t| matches!(t, Trigger::SiegeDefeated));
            game.pending.extend(fired);
        }
    }
    Some(())
}
