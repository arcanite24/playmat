//! Genesis Wave: "Reveal the top X cards of your library. You may put any number of permanent cards with mana value
//! X or less from among them onto the battlefield. Then put all cards revealed this way that weren't put onto the
//! battlefield into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GenesisWave.GenesisWaveEffect", apply)];

/// `gishath_suns_avatar::reveal_put_rest` with X cards, permanent cards of mana value X or less, the rest to the
/// graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0);
    super::gishath_suns_avatar::reveal_put_rest(
        game,
        d,
        cx,
        cx.controller(),
        x as usize,
        crate::card::PutCards::Graveyard,
        |g, c| g.obj(c).def.is_permanent_card() && (g.obj(c).def.mana_value() as i32) <= x,
    )
}
