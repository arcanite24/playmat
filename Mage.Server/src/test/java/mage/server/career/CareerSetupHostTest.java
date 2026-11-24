package mage.server.career;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.Assert;
import org.junit.Test;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;

/**
 * What a Career setup tells the Rust host (docs/rust/GameHost.md, Career tables): the JSON is the contract with
 * playmat-host's CareerSetup, which reads the same fixture (rust/crates/playmat-host/tests/fixtures/career-setup.json).
 * Set -Dxmage.updateHostFixtures=true to write the fixture again after a change on purpose.
 */
public class CareerSetupHostTest {

    private static final Path FIXTURE = Paths.get("..", "rust", "crates", "playmat-host", "tests", "fixtures", "career-setup.json");

    static CareerSetup sample() {
        CareerSetup.CareerTwists opponent = new CareerSetup.CareerTwists();
        opponent.lifeBonus = 5;
        opponent.extraCards = 1;
        opponent.permanents = Arrays.asList("Llanowar Elves|tapped", "Runeclaw Bear");
        CareerSetup.CareerTwists you = new CareerSetup.CareerTwists();
        you.startingLife = 25;
        you.extraCards = 2;
        CareerSetup.Personality personality = new CareerSetup.Personality();
        personality.aggression = 60;
        personality.risk = -20;
        personality.patience = 10;
        return CareerSetup.withPersonality(CareerSetup.ofRun(opponent, you), personality);
    }

    @Test
    public void theHostGetsTheFixture() throws Exception {
        CareerSetup setup = sample();
        Assert.assertNull(setup.hostProblem());
        Assert.assertEquals(Arrays.asList("Llanowar Elves", "Runeclaw Bear"), setup.hostCards());
        JsonObject json = setup.hostSetup();
        if (Boolean.getBoolean("xmage.updateHostFixtures")) {
            Files.write(FIXTURE, (new com.google.gson.GsonBuilder().setPrettyPrinting().create().toJson(json) + "\n")
                    .getBytes(StandardCharsets.UTF_8));
        }
        JsonObject fixture = JsonParser.parseString(new String(Files.readAllBytes(FIXTURE), StandardCharsets.UTF_8)).getAsJsonObject();
        Assert.assertEquals("the Rust host's fixture; rerun with -Dxmage.updateHostFixtures=true after a change on purpose", fixture, json);
    }

    @Test
    public void setupsTheHostLacksStayOnJava() {
        CareerSetup.CareerTwists emblem = new CareerSetup.CareerTwists();
        emblem.emblem = "anthem";
        Assert.assertNotNull(CareerSetup.ofTwists(emblem).hostProblem());
        CareerSetup.CareerTwists timed = new CareerSetup.CareerTwists();
        timed.timed = Collections.singletonList(new CareerSetup.TimedTwist());
        Assert.assertNotNull(CareerSetup.ofTwists(timed).hostProblem());
    }

    /**
     * Every twist field is one the host plays (life, life bonus, extra cards, permanents) or one hostProblem keeps on
     * Java (emblem, timed): a new field fails here until the host plays it or hostProblem names it.
     */
    @Test
    public void everyTwistIsPlayedOrRefused() {
        java.util.Set<String> fields = new java.util.TreeSet<>();
        for (java.lang.reflect.Field f : CareerSetup.CareerTwists.class.getFields()) {
            if (!java.lang.reflect.Modifier.isStatic(f.getModifiers())) {
                fields.add(f.getName());
            }
        }
        Assert.assertEquals(new java.util.TreeSet<>(Arrays.asList(
                "startingLife", "lifeBonus", "extraCards", "permanents", "emblem", "timed")), fields);
    }
}
