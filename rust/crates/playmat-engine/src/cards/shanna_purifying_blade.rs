//! Shanna, Purifying Blade: "Lifelink. At the beginning of your end step, you may pay {X}. If you do, draw X cards.
//! X can't be greater than the amount of life you gained this turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShannaPurifyingBlade.ShannaPurifyingBladeEffect", apply)];

/// No life gained this turn: nothing. Else `ManaUtil.playerPaysXGenericMana(true, …, lifeGained)`: X announced
/// ("Choose how much mana to pay", 0 to the life gained) and paid as {X} ("<player> pays {X}."; a failed payment
/// undoes itself and pays 0), then X cards drawn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let gained = game.player(you).life_gained;
    if gained < 1 {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: gained }) {
            Answer::Int(n) if (0..=gained).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if x > 0 {
        let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
        let before = game.clone();
        if !pay_mana(game, d, you, &cost) {
            if d.stopped() {
                return None;
            }
            *game = before;
            return Some(());
        }
    }
    if log::kept(game) {
        let line = format!("{} pays {{{x}}}.", log::player(you));
        log::inform(game, line);
    }
    if x > 0 {
        draw(game, you, x as usize);
    }
    Some(())
}
