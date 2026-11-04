package mage.server.platform;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.server.career.CareerContent;
import mage.server.career.CareerCosmetics;
import mage.server.websocket.rpc.JsonCodec;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

import static org.assertj.core.api.Assertions.assertThat;

/** The views Rust serves are the bridge's answers to careerStarters and careerCosmetics, written whole. */
public class CareerViewsExportTest {

    @TempDir
    File dir;

    @Test
    void writesTheBridgesAnswers() throws Exception {
        File file = new File(dir, "views/career-views.json");
        CareerViewsExport.write(file);
        JsonObject views = JsonParser.parseString(new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8)).getAsJsonObject();
        assertThat(views.get("starters")).isEqualTo(JsonCodec.GSON.toJsonTree(CareerContent.get().starters()));
        assertThat(views.get("cosmetics")).isEqualTo(JsonCodec.GSON.toJsonTree(CareerCosmetics.get().items()));
        assertThat(views.getAsJsonArray("starters").size()).isGreaterThan(3);
        assertThat(new File(dir, "views/career-views.json.part")).doesNotExist();

        assertThat(views.getAsJsonArray("quests").size()).isEqualTo(mage.server.career.CareerProgress.get().questTemplates().size());
        assertThat(views.getAsJsonArray("achievements").size()).isEqualTo(mage.server.career.CareerProgress.get().achievementTemplates().size());
        assertThat(views.getAsJsonObject("styles").size()).isPositive();
        // every starter's cards, without basics
        com.google.gson.JsonObject starterCards = views.getAsJsonObject("starterCards");
        assertThat(starterCards.size()).isEqualTo(CareerContent.get().starters().size());
        String starter = CareerContent.get().starters().get(0).id;
        assertThat(starterCards.getAsJsonArray(starter).size()).isPositive();
        starterCards.getAsJsonArray(starter).forEach(card -> assertThat(card.getAsJsonObject().get("rarity").getAsString()).isNotEqualTo("land"));
        // the roster: every opponent, as a player without wins sees it and as their nemesis
        com.google.gson.JsonArray roster = views.getAsJsonArray("opponents");
        assertThat(roster.size()).isEqualTo(CareerContent.get().opponents().size());
        JsonObject first = roster.get(0).getAsJsonObject();
        assertThat(first.getAsJsonObject("view").get("tier").getAsInt()).isEqualTo(1);
        assertThat(first.getAsJsonObject("view").get("unlocked").getAsBoolean()).isTrue();
        assertThat(first.getAsJsonObject("view").get("nemesis").getAsBoolean()).isFalse();
        assertThat(first.getAsJsonObject("nemesis").get("nemesis").getAsBoolean()).isTrue();
        assertThat(first.getAsJsonObject("nemesis").get("bounty").getAsInt()).isPositive();
        assertThat(first.getAsJsonObject("nemesis").getAsJsonObject("twists").get("lifeBonus").getAsInt()).isPositive();
    }
}
