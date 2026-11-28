//! Branch of Vitu-Ghazi: "When Branch of Vitu-Ghazi is turned face up, add two mana of any one color. Until end of
//! turn, you don't lose this mana as steps and phases end."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BranchOfVituGhazi.BranchOfVituGhaziEffect", apply)];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// `ChoiceColor(true, "Choose a color of mana to add")`, then two of it, kept in the pool until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let choices: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Choose a color of mana to add".into(), choices };
    let k = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => {
                if let Some(k) = NAMES.iter().position(|n| *n == t) {
                    break k;
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
    };
    let p = game.player_mut(you);
    p.mana_pool.colored[k] += 2;
    p.eot_mana.colored[k] += 2;
    Some(())
}
