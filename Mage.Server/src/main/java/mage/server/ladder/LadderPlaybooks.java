package mage.server.ladder;

import com.google.gson.Gson;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Loads `/ladder/playbooks.json` (kept in sync with `rust/crates/playmat-ai/src/playbooks.json`), providing
 * deck metadata (archetype, tagline, keyCards) and Bo3 sideboard plans for the server.
 */
public final class LadderPlaybooks {

    private static final Logger logger = Logger.getLogger(LadderPlaybooks.class);
    private static final String FILE = "/ladder/playbooks.json";

    public static class SideboardSwap {
        public Map<String, Integer> in;
        public Map<String, Integer> out;
    }

    public static class DeckPlaybook {
        public String profile;
        public String archetype;
        public String tagline;
        public List<String> keyCards;
        public Map<String, SideboardSwap> sideboard;
    }

    private static class Bundle {
        Map<String, Object> profiles;
        Map<String, DeckPlaybook> decks;
    }

    private static volatile LadderPlaybooks instance;

    private final Map<String, DeckPlaybook> decks;

    LadderPlaybooks(Reader reader) {
        Bundle b = new Gson().fromJson(reader, Bundle.class);
        Map<String, DeckPlaybook> map = new LinkedHashMap<>();
        if (b != null && b.decks != null) {
            map.putAll(b.decks);
        }
        this.decks = Collections.unmodifiableMap(map);
    }

    public static LadderPlaybooks get() {
        LadderPlaybooks current = instance;
        if (current == null) {
            synchronized (LadderPlaybooks.class) {
                current = instance;
                if (current == null) {
                    try (InputStream in = LadderPlaybooks.class.getResourceAsStream(FILE)) {
                        if (in == null) {
                            logger.warn("Ladder playbooks missing at " + FILE);
                            current = new LadderPlaybooks(new java.io.StringReader("{}"));
                        } else {
                            current = new LadderPlaybooks(new InputStreamReader(in, StandardCharsets.UTF_8));
                        }
                    } catch (IOException | RuntimeException e) {
                        logger.warn("Ladder playbooks couldn't be read: " + e.getMessage());
                        current = new LadderPlaybooks(new java.io.StringReader("{}"));
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** Looks up a deck playbook by id (strip optional `@hash` suffix). Returns null for profile-only or unknown ids. */
    public DeckPlaybook deck(String playbookId) {
        if (playbookId == null || playbookId.isEmpty() || playbookId.startsWith("profile:")) {
            return null;
        }
        int at = playbookId.indexOf('@');
        String key = at >= 0 ? playbookId.substring(0, at) : playbookId;
        return decks.get(key);
    }

    public Map<String, DeckPlaybook> decks() {
        return decks;
    }
}
