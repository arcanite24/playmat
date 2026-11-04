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
    }
}
