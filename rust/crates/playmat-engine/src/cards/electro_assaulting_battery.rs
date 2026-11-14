//! Electro, Assaulting Battery: "When Electro leaves the battlefield, you may pay {X}. When you do, he deals X damage
//! to target player."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("ElectroAssaultingBattery.ElectroAssaultingBatteryEffect", apply)];

/// "Pay x mana to deal x damage to target player?"; yes: a player chosen (`TargetPlayer.chooseTarget`), then
/// `ManaUtil.playerPaysXGenericMana`: X announced and paid as generic mana ("<player> pays {X}."), and that much
/// damage to the player (none when the payment fails)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Pay x mana to deal x damage to target player?")? {
        return Some(());
    }
    let players: Vec<Seat> = (0..game.seats()).filter(|p| !game.player(*p).lost).collect();
    let target = loop {
        match ask(game, d, you, &Query::Target { players: players.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if players.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
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
    deal_damage(game, cx.source_id(), Target::Player(target), x, false);
    Some(())
}
