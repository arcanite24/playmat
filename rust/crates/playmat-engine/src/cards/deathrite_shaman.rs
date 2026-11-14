//! Deathrite Shaman: "{T}: Exile target land card from a graveyard. Add one mana of any color." Not a mana ability
//! (it targets): `AddManaOfAnyColorEffect(1, new LimitedDynamicValue(1, …), false)`, whose dynamic value is only
//! the available-mana estimate, so it always adds one mana.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeathriteShaman.AddManaOfAnyColorEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &[EffectDef::AddManaOfAnyColorEffect { amount: 1 }]);
    if d.stopped() {
        return None;
    }
    Some(())
}
