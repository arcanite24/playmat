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

    static CareerSetup.TimedTwist timed(String when, Integer n, String card) {
        CareerSetup.TimedTwist twist = new CareerSetup.TimedTwist();
        twist.when = when;
        if (CareerSetup.TimedTwist.TURN.equals(when)) {
            twist.turn = n;
        } else if (CareerSetup.TimedTwist.LIFE.equals(when)) {
            twist.life = n;
        }
        twist.card = card;
        return twist;
    }

    static CareerSetup sample() {
        CareerSetup.CareerTwists opponent = new CareerSetup.CareerTwists();
        opponent.lifeBonus = 5;
        opponent.extraCards = 1;
        opponent.permanents = Arrays.asList("Llanowar Elves|tapped", "Runeclaw Bear");
        opponent.emblem = "Glorious Anthem";
        // a boss's phases: life thresholds with an edge, a life gain; one not well formed, which Java skips
        CareerSetup.TimedTwist phase = timed(CareerSetup.TimedTwist.LIFE, 12, null);
        phase.then = new CareerSetup.CareerTwists();
        phase.then.extraCards = 2;
        phase.then.permanents = Collections.singletonList("Runeclaw Bear|tapped");
        phase.message = "Phase two: the boss draws two.";
        CareerSetup.TimedTwist wind = timed(CareerSetup.TimedTwist.LIFE, 6, null);
        wind.gainLife = 5;
        wind.message = "Second wind: 5 life.";
        CareerSetup.TimedTwist broken = timed(CareerSetup.TimedTwist.TURN, 0, null);
        broken.gainLife = 1;
        CareerSetup.TimedTwist cast = timed(CareerSetup.TimedTwist.CAST, null, "Lightning Bolt");
        cast.gainLife = -2;
        opponent.timed = Arrays.asList(phase, wind, broken, cast);
        CareerSetup.CareerTwists you = new CareerSetup.CareerTwists();
        you.startingLife = 25;
        you.extraCards = 2;
        CareerSetup.TimedTwist turn = timed(CareerSetup.TimedTwist.TURN, 5, null);
        turn.gainLife = 5;
        turn.message = "Second Wind: 5 life.";
        you.timed = Collections.singletonList(turn);
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
        Assert.assertEquals(Arrays.asList("Llanowar Elves", "Runeclaw Bear", "Glorious Anthem", "Runeclaw Bear"), setup.hostCards());
        JsonObject json = setup.hostSetup();
        if (Boolean.getBoolean("xmage.updateHostFixtures")) {
            Files.write(FIXTURE, (new com.google.gson.GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create().toJson(json) + "\n")
                    .getBytes(StandardCharsets.UTF_8));
        }
        JsonObject fixture = JsonParser.parseString(new String(Files.readAllBytes(FIXTURE), StandardCharsets.UTF_8)).getAsJsonObject();
        Assert.assertEquals("the Rust host's fixture; rerun with -Dxmage.updateHostFixtures=true after a change on purpose", fixture, json);
    }

    /** the rules the emblem shows are Java's own, as the twist's ability writes them */
    @Test
    public void theTimedTwistsCarryJavasRules() {
        JsonObject json = sample().hostSetup();
        com.google.gson.JsonArray timed = json.getAsJsonObject("opponent").getAsJsonArray("timed");
        Assert.assertEquals("the twist that isn't well formed is skipped", 3, timed.size());
        Assert.assertEquals("When this player's life is 12 or less, Phase two: the boss draws two. This ability triggers only once.",
                timed.get(0).getAsJsonObject().get("rule").getAsString());
        Assert.assertEquals("opponent", timed.get(0).getAsJsonObject().get("who").getAsString());
        Assert.assertEquals("you", timed.get(2).getAsJsonObject().get("who").getAsString());
        Assert.assertEquals(" gets an edge: 2 extra cards, Runeclaw Bear.",
                timed.get(0).getAsJsonObject().getAsJsonObject("then").get("log").getAsString());
        com.google.gson.JsonObject turn = json.getAsJsonObject("you").getAsJsonArray("timed").get(0).getAsJsonObject();
        Assert.assertEquals("At the beginning of turn 5, Second Wind: 5 life. This ability triggers only once.",
                turn.get("rule").getAsString());
    }

    @Test
    public void setupsTheHostLacksStayOnJava() {
        // an emblem of a card the host hasn't checked from the command zone (the gauntlet's drum relic)
        CareerSetup.CareerTwists emblem = new CareerSetup.CareerTwists();
        emblem.emblem = "Fervor";
        Assert.assertEquals("a Career emblem the host lacks (Fervor)", CareerSetup.ofTwists(emblem).hostProblem());
        // and one a timed twist brings in
        CareerSetup.CareerTwists later = new CareerSetup.CareerTwists();
        CareerSetup.TimedTwist twist = timed(CareerSetup.TimedTwist.LIFE, 6, null);
        twist.then = emblem;
        later.timed = Collections.singletonList(twist);
        Assert.assertEquals("a Career emblem the host lacks (Fervor)", CareerSetup.ofTwists(later).hostProblem());
        // a card a timed twist puts in play must be verified for Rust, as the starting permanents are
        CareerSetup.CareerTwists crusher = new CareerSetup.CareerTwists();
        CareerSetup.TimedTwist spawn = timed(CareerSetup.TimedTwist.LIFE, 50, null);
        spawn.then = new CareerSetup.CareerTwists();
        spawn.then.permanents = Collections.singletonList("Ulamog's Crusher");
        crusher.timed = Collections.singletonList(spawn);
        Assert.assertNull(CareerSetup.ofTwists(crusher).hostProblem());
        Assert.assertEquals(Collections.singletonList("Ulamog's Crusher"), CareerSetup.ofTwists(crusher).hostCards());
    }

    /**
     * Every twist field is one the host plays (life, life bonus, extra cards, permanents, an emblem, timed twists): a
     * new field fails here until the host plays it or hostProblem names it.
     */
    @Test
    public void everyTwistIsPlayedOrRefused() {
        Assert.assertEquals(new java.util.TreeSet<>(Arrays.asList(
                "startingLife", "lifeBonus", "extraCards", "permanents", "emblem", "timed")), fields(CareerSetup.CareerTwists.class));
        Assert.assertEquals(new java.util.TreeSet<>(Arrays.asList(
                "when", "turn", "life", "card", "who", "then", "gainLife", "message")), fields(CareerSetup.TimedTwist.class));
    }

    private static java.util.Set<String> fields(Class<?> type) {
        java.util.Set<String> fields = new java.util.TreeSet<>();
        for (java.lang.reflect.Field f : type.getFields()) {
            if (!java.lang.reflect.Modifier.isStatic(f.getModifiers())) {
                fields.add(f.getName());
            }
        }
        return fields;
    }
}
