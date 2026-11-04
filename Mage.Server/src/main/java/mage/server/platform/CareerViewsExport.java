package mage.server.platform;

import mage.server.career.CareerContent;
import mage.server.career.CareerCosmetics;
import mage.server.career.CareerService;
import mage.server.websocket.rpc.JsonCodec;
import org.apache.log4j.Logger;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Writes the Career's content views the Rust platform service answers (docs/rust/Career.md): the starter decks, the
 * cosmetics catalog and the opponents roster, as the bridge sends them. They are built from the content files, the capsule machines, the
 * achievements, the campaigns and staff's content packs, so Java builds them and Rust serves what it wrote.
 * <p>
 * {@code XMAGE_CAREER_EXPORT=<file>} (or {@code -Dxmage.career.export}): written after startup and after staff reload
 * the content packs, to a temporary file first and then moved in place.
 */
public final class CareerViewsExport {

    private static final Logger logger = Logger.getLogger(CareerViewsExport.class);

    public static final String PROP = "xmage.career.export";

    private CareerViewsExport() {
    }

    static String target() {
        String path = System.getProperty(PROP, System.getenv("XMAGE_CAREER_EXPORT"));
        return path == null || path.trim().isEmpty() ? null : path.trim();
    }

    /** writes the views when a file is asked for; a failure is logged and Rust leaves these calls to Java */
    public static void writeIfAsked() {
        String path = target();
        if (path == null) {
            return;
        }
        try {
            write(new File(path));
        } catch (IOException | RuntimeException e) {
            logger.warn("Career views can't be written to " + path + ": " + e.getMessage());
        }
    }

    /**
     * The roster in the order players see it: each opponent as a player without wins sees it, the same as their
     * nemesis, and the opponent whose win opens a rival chapter ({@code CareerLadder.rivalOpen}). Rust adds the
     * player's wins, the unlocks and the nemesis.
     */
    static List<Map<String, Object>> opponents(CareerContent content) {
        Map<String, Integer> none = Collections.emptyMap();
        Map<Integer, Integer> noTiers = Collections.emptyMap();
        List<Map<String, Object>> roster = new ArrayList<>();
        for (CareerContent.Opponent opponent : CareerService.ordered(content)) {
            Map<String, Object> entry = new LinkedHashMap<>();
            entry.put("view", CareerService.opponentView(content, opponent, none, noTiers, false));
            entry.put("nemesis", CareerService.opponentView(content, opponent, none, noTiers, true));
            if (opponent.rival != null && opponent.rival.chapter > 1) {
                for (CareerContent.Opponent other : content.opponents()) {
                    if (other.rival != null && other.rival.chapter == opponent.rival.chapter - 1) {
                        entry.put("after", other.id);
                        break;
                    }
                }
            }
            roster.add(entry);
        }
        return roster;
    }

    static void write(File file) throws IOException {
        Map<String, Object> views = new LinkedHashMap<>();
        views.put("starters", CareerContent.get().starters());
        views.put("cosmetics", CareerCosmetics.get().items());
        views.put("opponents", opponents(CareerContent.get()));
        views.put("achievements", mage.server.career.CareerProgress.get().achievementExport());
        views.put("quests", new ArrayList<>(mage.server.career.CareerProgress.get().questTemplates().values()));
        File parent = file.getAbsoluteFile().getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }
        File part = new File(file.getPath() + ".part");
        Files.write(part.toPath(), JsonCodec.GSON.toJson(views).getBytes(StandardCharsets.UTF_8));
        Files.move(part.toPath(), file.toPath(), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        logger.info("Career views written to " + file);
    }
}
