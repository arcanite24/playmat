//! Delina, Wild Mage: "Whenever Delina, Wild Mage attacks, choose target creature you control, then roll a d20.
//! 1-14 | Create a tapped and attacking token that's a copy of that creature, except it's not legendary and it has
//! "At end of combat, exile this token."
//! 15-20 | Create one of those tokens. You may roll again."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DelinaWildMage.DelinaWildMageEffect", apply).effects(&[
    "new CreateTokenCopyTargetEffect(null, null, false, 1, true, true).setIsntLegendary(true).addAdditionalAbilities(new EndOfCombatTriggeredAbility(new ExileSourceEffect(), false))",
])];

/// `rollDice(…, 20)` (`RandomUtil.nextInt(20) + 1`), then the token copy of the target (as it last was), again while
/// the roll is 15 or more and its controller says "Roll again?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let of = match cx.item.targets.first().and_then(|t| t.first()) {
        Some(Target::Obj(o)) => *o,
        _ => return Some(()),
    };
    loop {
        let result = game.rng.next_int_bound(20) + 1;
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(of)]);
        if d.stopped() {
            return None;
        }
        if result < 15 || !choose_use(game, d, you, "Roll again?")? {
            return Some(());
        }
    }
}
