//! Cerebral Download: "Surveil X, where X is the number of artifacts you control. Then draw three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CerebralDownload.CerebralDownloadEffect", apply)
    .filters(&["StaticFilters.FILTER_CONTROLLED_PERMANENT_ARTIFACT"])];

/// `player.surveil(ArtifactYouControlCount)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let amount = crate::card::Amount::Of(crate::card::Value::Permanents { filter: cx.filters[0].clone() });
    cx.run(game, d, &[EffectDef::SurveilEffect { amount }]);
    (!d.stopped()).then_some(())
}
