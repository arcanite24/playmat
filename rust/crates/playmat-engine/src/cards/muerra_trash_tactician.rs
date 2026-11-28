//! Muerra, Trash Tactician: `AddManaInAnyCombinationEffect(amount, netAmount, symbol, symbol)` resolving: "add {R}
//! or {G} for each Raccoon you control" (Muerra, Trash Tactician's first main phase trigger)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MuerraTrashTactician.AddManaInAnyCombinationEffect", apply).args(&[
    "amount",
    "amount",
    "one_of:ColoredManaSymbol.W|ColoredManaSymbol.U|ColoredManaSymbol.B|ColoredManaSymbol.R|ColoredManaSymbol.G",
    "one_of:ColoredManaSymbol.W|ColoredManaSymbol.U|ColoredManaSymbol.B|ColoredManaSymbol.R|ColoredManaSymbol.G",
])];

/// `produceMana`: `getMultiAmount(…, 0, amount, amount, MANA)` over the symbols in their order, then the mana goes
/// to the controller's pool (nothing asked when it's 0)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_amount(game, 0)?.max(0);
    let colors: Vec<Color> = cx.args[2..]
        .iter()
        .filter_map(|a| match a.trim().rsplit('.').next()? {
            "W" => Some(Color::W),
            "U" => Some(Color::U),
            "B" => Some(Color::B),
            "R" => Some(Color::R),
            "G" => Some(Color::G),
            _ => None,
        })
        .collect();
    if n == 0 || colors.is_empty() {
        return Some(());
    }
    let mut defaults = vec![0; colors.len()];
    defaults[0] = n;
    let ranges = vec![(0, n); colors.len()];
    let values = multi_amount(game, d, cx.controller(), defaults, ranges, (n, n), (None, Vec::new()))?;
    let pool = &mut game.player_mut(cx.controller()).mana_pool;
    for (c, v) in colors.iter().zip(values) {
        pool.colored[c.index()] += v.max(0) as u32;
    }
    Some(())
}
