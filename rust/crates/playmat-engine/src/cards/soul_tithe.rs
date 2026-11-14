//! Soul Tithe: "Enchant nonland permanent. At the beginning of the upkeep of enchanted permanent's controller, that
//! player sacrifices it unless they pay {X}, where X is its mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoulTithe.SoulTitheEffect", apply)];

/// The Aura's permanent's controller is asked (`chooseUse`) to pay {X}; not paid, they sacrifice it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = cx.source_id();
    if game.obj(aura).zone != Zone::Battlefield {
        return Some(());
    }
    let Some((p, zcc)) = game.obj(aura).attached_to else { return Some(()) };
    if game.obj(p).zone != Zone::Battlefield || game.obj(p).zcc != zcc {
        return Some(());
    }
    let payer = game.obj(p).controller;
    let cost = crate::card::ManaCost::parse(&format!("{{{}}}", game.obj(p).def.mana_value())).ok()?;
    let message = format!("Pay {} for {}? (otherwise you sacrifice it)", cost.text, game.obj(p).name());
    if choose_use(game, d, payer, &message)? {
        let paid = pay_mana(game, d, payer, &cost);
        if d.stopped() {
            return None;
        }
        if paid {
            return Some(());
        }
    }
    sacrifice(game, p);
    Some(())
}
