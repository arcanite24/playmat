//! Skrelv, Defector Mite: "{W/P}, {T}: Choose a color. Another target creature you control gains toxic 1 and
//! hexproof from that color until end of turn. It can't be blocked by creatures of that color this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkrelvDefectorMite.SkrelvDefectorMiteEffect", apply).effects(&[
    "new GainAbilityTargetEffect(new ToxicAbility(1), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HexproofFromWhiteAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HexproofFromBlueAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HexproofFromBlackAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HexproofFromRedAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HexproofFromGreenAbility.getInstance(), Duration.EndOfTurn)",
])];

const NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];
const CODES: [&str; 5] = ["W", "U", "B", "R", "G"];

/// `ChoiceColor(true)` ("Choose color"), then toxic 1, hexproof from that color and "can't be blocked by creatures of
/// that color" (`CantBeBlockedByAllTargetEffect` with a `FilterCreaturePermanent` of the color), each until end of
/// turn on the target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
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
        if d.stopped() {
            return None;
        }
    };
    let unblockable: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "GainAbilityTargetEffect",
        "ability": {"class": "CantBeBlockedByAbility",
            "filter": [{"p": "type", "t": "CREATURE"}, {"p": "color", "c": CODES[k]}]},
        "duration": "EndOfTurn"}))
    .ok()?;
    cx.run(game, d, &[cx.effects[0].clone(), cx.effects[1 + k].clone(), unblockable]);
    Some(())
}
