//! Spell Rupture: "Counter target spell unless its controller pays {X}, where X is the greatest power among creatures
//! you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpellRupture.SpellRuptureCounterUnlessPaysEffect", apply)
    .effects(&["new CounterTargetEffect()"])];

/// X worked out as it resolves (`GreatestAmongPermanentsValue.POWER_CONTROLLED_CREATURES`), then the spell's
/// controller is asked "Pay {X}, where X is N? (otherwise <spell> will be countered)" (asked even when they can't
/// pay) and pays; otherwise the spell is countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(payer) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).map(|i| i.controller)
    else {
        return Some(());
    };
    let max_power = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0)
        .max(0);
    let Ok(cost) = ManaCost::parse(&format!("{{{max_power}}}")) else { return Some(()) };
    let message = format!(
        "Pay {}, where X is {}? (otherwise {} will be countered)",
        cost.text,
        max_power,
        game.obj(spell).name()
    );
    if choose_use(game, d, payer, &message)? {
        let paid = pay_mana(game, d, payer, &cost);
        if d.stopped() {
            return None;
        }
        if paid {
            return Some(());
        }
    }
    if log::kept(game) {
        let name = log::source(game).map(|s| game.obj(s).name()).unwrap_or_default();
        let message = format!("{}: cost wasn't payed - countering {}", name, game.obj(spell).name());
        log::inform(game, message);
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
