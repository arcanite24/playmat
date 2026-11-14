//! Heraldic Banner: "As Heraldic Banner enters, choose a color. Creatures you control of the chosen color get +1/+0.
//! {T}: Add one mana of the chosen color."

use super::*;
use crate::card::{AbilityDef, Color};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("HeraldicBanner.ChooseColorEffect", choose_color).args(&["one_of:Outcome.Benefit"]),
    Handler::continuous("HeraldicBanner.HeraldicBannerEffect", of_chosen_color, Layer::ModifyPt(plus_one)),
];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];

/// the color chosen as `id` entered, kept as one of its marks while it stays on the battlefield
pub(crate) fn chosen_color(game: &Game, id: ObjId) -> Option<Color> {
    let o = game.obj(id);
    let m = o.marks.iter().find_map(|m| m.strip_prefix("chosen color "))?;
    NAMES.iter().position(|n| *n == m).map(|k| Color::ALL[k])
}

/// `ChooseColorEffect` as the Banner enters: "Choose color"; its "{T}: Add one mana of the chosen color" becomes
/// that color's mana ability (`AddManaChosenColorEffect`)
fn choose_color(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    let choices: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    let q = Query::Choice { message: "Choose color".into(), choices };
    let k = loop {
        match ask(game, d, cx.controller(), &q) {
            Answer::Text(t) => {
                if let Some(k) = NAMES.iter().position(|n| *n == t) {
                    break k;
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
    };
    let c = Color::ALL[k];
    let mut def = (*game.obj(id).def).clone();
    for a in def.abilities.iter_mut() {
        if let AbilityDef::ChosenColorManaAbility { amount, rule, .. } = a {
            *a = AbilityDef::ColoredManaAbility {
                color: c,
                amount: *amount,
                effects: Vec::new(),
                cost: Vec::new(),
                no_tap: false,
                creature_spell_haste: false,
                net: None,
                rule: rule.clone(),
            };
        }
    }
    let o = game.obj_mut(id);
    o.marks.retain(|m| !m.starts_with("chosen color "));
    o.marks.push(format!("chosen color {}", NAMES[k]));
    if o.printed_def.is_none() {
        o.printed_def = Some(o.def.clone());
    }
    o.def = Arc::new(def);
    o.add_info("chosen color", &format!("Chosen color: {}", NAMES[k]));
    Some(())
}

/// creatures you control of the chosen color
fn of_chosen_color(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let Some(c) = chosen_color(game, cx.source) else { return false };
    game.obj(id).controller == cx.controller && game.has_type(id, CardType::Creature) && game.colors(id).contains(&c)
}

fn plus_one(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (1, 0)
}
