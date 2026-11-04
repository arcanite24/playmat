package mage.server.platform;

import mage.server.career.CareerContent;
import mage.server.career.CareerCosmetics;
import mage.server.websocket.rpc.JsonCodec;
import org.apache.log4j.Logger;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Writes the Career's content views the Rust platform service answers (docs/rust/Career.md): the starter decks and
 * the cosmetics catalog, as the bridge sends them. They are built from the content files, the capsule machines, the
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

    static void write(File file) throws IOException {
        Map<String, Object> views = new LinkedHashMap<>();
        views.put("starters", CareerContent.get().starters());
        views.put("cosmetics", CareerCosmetics.get().items());
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
