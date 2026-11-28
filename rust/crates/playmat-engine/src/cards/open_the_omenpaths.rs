//! Open the Omenpaths: "Choose one — • Add two mana of any one color and two mana of any other color. Spend this mana
//! only to cast creature or enchantment spells. • Creatures you control get +1/+0 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OpenTheOmenpaths.OpenTheOmenpathsEffect", apply)];

/// `ManaChoice.chooseTwoDifferentColors` ("Choose color 1", then "Choose color 2" of the others), doubled, added as
/// conditional mana spent only to cast creature or enchantment spells (`OpenTheOmenpathsCondition`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // (nothing is being paid: each color is asked)
    let owed = ManaCost { colored: [1; 5], ..ManaCost::default() };
    let all = Color::ALL.to_vec();
    let a = pick_color(game, d, you, &owed, &all, "Choose color 1")?;
    let rest: Vec<Color> = all.into_iter().filter(|c| *c != a).collect();
    let b = pick_color(game, d, you, &owed, &rest, "Choose color 2")?;
    let mut m = Mana::of(a, 2);
    m.colored[b.index()] += 2;
    let spec = serde_json::json!({
        "class": "ConditionalManaAbility",
        "filter": [{"p": "or", "xs": [{"p": "type", "t": "CREATURE"}, {"p": "type", "t": "ENCHANTMENT"}]}],
    });
    let Ok(mut ability) = serde_json::from_value::<AbilityDef>(spec) else { return Some(()) };
    if let AbilityDef::ConditionalManaAbility { mana, .. } = &mut ability {
        *mana = Some(m);
    }
    let k = game.effect_mana.len();
    Arc::make_mut(&mut game.effect_mana).push(ability);
    game.player_mut(you).conditional_pool.push((m, cx.source_id(), EFFECT_MANA + k));
    Some(())
}
