//! Vraska, Relic Seeker: "+2: Create a 2/2 black Pirate creature token with menace. −3: Destroy target artifact,
//! creature, or enchantment. Create a Treasure token. −10: Target player's life total becomes 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VraskaRelicSeeker.VraskaRelicSeekerLifeTotalEffect", apply)];

/// `PlayerImpl.setLife(1)`: the difference gained or lost
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let life = game.player(p).life;
    if life < 1 {
        gain_life(game, p, 1 - life);
    } else if life > 1 {
        lose_life(game, p, life - 1);
    }
    Some(())
}
