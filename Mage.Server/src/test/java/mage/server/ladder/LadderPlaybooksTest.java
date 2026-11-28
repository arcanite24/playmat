package mage.server.ladder;

import org.junit.Assert;
import org.junit.Test;

public class LadderPlaybooksTest {

    @Test
    public void commanderProfilesLoad() {
        LadderPlaybooks playbooks = LadderPlaybooks.get();
        String[] expected = {
                "cmdr_voltron",
                "cmdr_tokens",
                "cmdr_ramp",
                "cmdr_aristocrats",
                "cmdr_spellslinger",
                "cmdr_tribal",
                "cmdr_reanimator",
                "cmdr_lifegain",
                "cmdr_artifacts",
                "cmdr_control"
        };
        for (String id : expected) {
            LadderPlaybooks.Profile profile = playbooks.profile(id);
            Assert.assertNotNull("Missing commander playbook profile: " + id, profile);
            Assert.assertNotNull("Missing combat hints for profile: " + id, profile.combat);
            Assert.assertTrue(
                    "Commander profile should protect commander role: " + id,
                    profile.combat.protectRoles != null && profile.combat.protectRoles.contains("commander")
            );
            Assert.assertSame(profile, playbooks.profile("profile:" + id));
        }
    }
}
