package mage.server.admin;

import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Server settings staff change from /admin without a restart. A setting that was never changed follows the server's
 * config (config.xml and the environment); changing it back to empty returns it there.
 */
public final class RuntimeSettings {

    private static final Logger logger = Logger.getLogger(RuntimeSettings.class);

    /** "open": anyone may make an account; "invite": only with an invite link, or when already on the allowlist */
    public static final String REGISTRATION = "registration";
    /** "on" or "off"; the environment's XMAGE_CAREER=off still turns Career off for good */
    public static final String CAREER = "career";
    /** how many AI opponents one table may seat */
    public static final String MAX_AI_OPPONENTS = "maxAiOpponents";
    /** how long a player may take to answer before they lose the game */
    public static final String MAX_SECONDS_IDLE = "maxSecondsIdle";

    /** how many AI-against-AI batch games run at once; 0 stops the batch runner */
    public static final String BATCH_CONCURRENCY = "batchConcurrency";
    /** how many batch games one player may ask for in a day */
    public static final String BATCH_GAMES_PER_DAY = "batchGamesPerDay";
    public static final int BATCH_CONCURRENCY_DEFAULT = 1;
    public static final int BATCH_GAMES_PER_DAY_DEFAULT = 200;

    /** Discord posts, each "on" or "off": tables opening, events starting, matches starting, staff news */
    public static final String DISCORD_TABLES = "discordTables";
    public static final String DISCORD_EVENTS = "discordEvents";
    public static final String DISCORD_MATCHES = "discordMatches";
    public static final String DISCORD_STAFF = "discordStaff";

    public static final List<String> NAMES = Collections.unmodifiableList(Arrays.asList(REGISTRATION, CAREER, MAX_AI_OPPONENTS, MAX_SECONDS_IDLE,
            BATCH_CONCURRENCY, BATCH_GAMES_PER_DAY,
            DISCORD_TABLES, DISCORD_EVENTS, DISCORD_MATCHES, DISCORD_STAFF));

    private static final String PREFIX = "runtime.";
    private static final String EMPTY = "";
    private static volatile RuntimeSettings instance;

    private final AdminStore store;
    /** stored values; EMPTY when the setting follows the config */
    private final Map<String, String> cache = new ConcurrentHashMap<>();

    RuntimeSettings(AdminStore store) {
        this.store = store;
    }

    public static RuntimeSettings get() {
        RuntimeSettings current = instance;
        if (current == null) {
            synchronized (RuntimeSettings.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new RuntimeSettings(AdminStore.get());
                    } catch (IllegalStateException e) {
                        // without the admin database every setting follows the config
                        return new RuntimeSettings(null);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests: the settings every server class sees (null goes back to the server's store). */
    public static void use(RuntimeSettings settings) {
        instance = settings;
    }

    public static RuntimeSettings create(AdminStore store) {
        return new RuntimeSettings(store);
    }

    /** The stored value, or null when the setting follows the config. */
    public String stored(String name) {
        if (store == null) {
            return null;
        }
        String value = cache.computeIfAbsent(name, key -> {
            try {
                String text = store.setting(PREFIX + key);
                return text == null ? EMPTY : text;
            } catch (SQLException e) {
                logger.error("Setting " + key + " can't be read; using the config", e);
                return EMPTY;
            }
        });
        return value.isEmpty() ? null : value;
    }

    public AdminSetting row(String name) throws SQLException {
        return store == null ? null : store.settingRow(PREFIX + name);
    }

    /**
     * Changes a setting; an empty value returns it to the config.
     *
     * @return the stored value, normalized, or null
     * @throws IllegalArgumentException for an unknown setting or a value it can't take
     */
    public String set(String name, String value, String by, long now) throws SQLException {
        String normalized = normalize(name, value);
        if (store == null) {
            throw new IllegalStateException("The admin database can't be opened");
        }
        store.putSetting(PREFIX + name, normalized == null ? EMPTY : normalized, by, now);
        cache.put(name, normalized == null ? EMPTY : normalized);
        return normalized;
    }

    static String normalize(String name, String value) {
        String text = value == null ? "" : value.trim().toLowerCase(Locale.ROOT);
        if (!NAMES.contains(name)) {
            throw new IllegalArgumentException("There is no setting named " + name);
        }
        if (text.isEmpty()) {
            return null;
        }
        switch (name) {
            case REGISTRATION:
                if (text.equals("open") || text.equals("invite")) {
                    return text;
                }
                throw new IllegalArgumentException("Registration is open or invite");
            case CAREER:
            case DISCORD_TABLES:
            case DISCORD_EVENTS:
            case DISCORD_MATCHES:
            case DISCORD_STAFF:
                if (text.equals("on") || text.equals("off")) {
                    return text;
                }
                throw new IllegalArgumentException(name + " is on or off");
            case MAX_AI_OPPONENTS:
                return String.valueOf(number(text, 0, 15, "AI opponents per table are 0 to 15"));
            case BATCH_CONCURRENCY:
                return String.valueOf(number(text, 0, 16, "Batch games at once are 0 to 16"));
            case BATCH_GAMES_PER_DAY:
                return String.valueOf(number(text, 0, 100000, "Batch games per player a day are 0 to 100000"));
            default:
                return String.valueOf(number(text, 30, 3600, "Time to answer is 30 to 3600 seconds"));
        }
    }

    private static int number(String text, int min, int max, String range) {
        try {
            int n = Integer.parseInt(text);
            if (n >= min && n <= max) {
                return n;
            }
        } catch (NumberFormatException ignored) {
        }
        throw new IllegalArgumentException(range);
    }

    public boolean registrationOpen() {
        return !"invite".equals(stored(REGISTRATION));
    }

    public boolean careerOn() {
        return !"off".equals(stored(CAREER));
    }

    /** whether one kind of Discord post is on (they all are until staff turn one off) */
    public boolean discord(String kind) {
        return !"off".equals(stored(kind));
    }

    /** the stored limit, or the config's (which may be null: no limit) */
    public String maxAiOpponents(String fromConfig) {
        String stored = stored(MAX_AI_OPPONENTS);
        return stored != null ? stored : fromConfig;
    }

    public int batchConcurrency() {
        String stored = stored(BATCH_CONCURRENCY);
        return stored != null ? Integer.parseInt(stored) : BATCH_CONCURRENCY_DEFAULT;
    }

    public int batchGamesPerDay() {
        String stored = stored(BATCH_GAMES_PER_DAY);
        return stored != null ? Integer.parseInt(stored) : BATCH_GAMES_PER_DAY_DEFAULT;
    }

    public int maxSecondsIdle(int fromConfig) {
        String stored = stored(MAX_SECONDS_IDLE);
        return stored != null ? Integer.parseInt(stored) : fromConfig;
    }
}
