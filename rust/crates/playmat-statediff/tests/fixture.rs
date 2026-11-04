//! The fixture the Java bridge (StateDiffTest) and the web client (statePatch.test.ts) share: real game views from
//! one game and the patch the Java bridge made between each two. The Rust port must make the same patches, written
//! out the same way, and applying them must give the next view exactly.

use serde_json::Value;

const FIXTURE: &str = include_str!("../../../../Mage.Web.Client/src/core/rpc/__fixtures__/gameStatePatches.json");

#[test]
fn same_patches_as_the_java_bridge() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let views = fixture["views"].as_array().unwrap();
    let patches = fixture["patches"].as_array().unwrap();
    assert_eq!(patches.len() + 1, views.len());
    for (i, expected) in patches.iter().enumerate() {
        let patch = playmat_statediff::diff(&views[i], &views[i + 1]);
        let written = patch.as_ref().map(|p| serde_json::to_string(p).unwrap());
        let expected_written = if expected.is_null() { None } else { Some(serde_json::to_string(expected).unwrap()) };
        assert_eq!(written, expected_written, "patch {i}");
        let applied = playmat_statediff::apply(Some(&views[i]), patch.as_ref()).unwrap();
        assert_eq!(serde_json::to_string(&applied).unwrap(), serde_json::to_string(&views[i + 1]).unwrap(), "view {}", i + 1);
    }
}
