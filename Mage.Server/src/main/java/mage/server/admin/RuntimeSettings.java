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
    /** a day's spending cap, in US cents, for the hybrid Archmage AI in Career's tier five (S050); 0 keeps it off */
    public static final String ARCHMAGE_DAILY_CENTS = "archmageDailyCents";
    /** progression pay outside Career's own tables, in percent of rewards.json: XP and coins */
    public static final String PROGRESS_XP_PERCENT = "progressXpPercent";
    public static final String PROGRESS_COIN_PERCENT = "progressCoinPercent";
    public static final int BATCH_CONCURRENCY_DEFAULT = 1;
    public static final int BATCH_GAMES_PER_DAY_DEFAULT = 200;

    /** Discord posts, each "on" or "off": tables opening, events starting, matches starting, staff news */
    public static final String DISCORD_TABLES = "discordTables";
    public static final String DISCORD_EVENTS = "discordEvents";
    public static final String DISCORD_MATCHES = "discordMatches";
    public static final String DISCORD_STAFF = "discordStaff";

    /** Support for the server (owner only): "on" or "off"; off hides the goal bar and the Support page */
    public static final String SUPPORT_ENABLED = "support.enabled";
    /** the monthly goal in US cents */
    public static final String SUPPORT_GOAL_CENTS = "support.goalCents";
    /** the Ko-fi page players support the server on; empty hides the support UI */
    public static final String SUPPORT_KOFI_URL = "support.kofiUrl";
    /** the GitHub Sponsors page, offered beside Ko-fi when set */
    public static final String SUPPORT_SPONSORS_URL = "support.sponsorsUrl";
    /** what the money pays for: lines separated by " · " or new lines, or a JSON list of strings */
    public static final String SUPPORT_COSTS = "support.costs";
    /** "on": the Discord thank-you post says the amount */
    public static final String SUPPORT_DISCORD_AMOUNTS = "support.discordAmounts";

    public static final long DEFAULT_GOAL_CENTS = 4000;
    public static final String DEFAULT_COSTS = "Server (Hetzner CAX21) $13.09 · Off-site backups · Mail · Domain · Buffer for busy months";
    static final int MAX_COSTS = 600;
    static final int MAX_URL = 200;

    public static final List<String> NAMES = Collections.unmodifiableList(Arrays.asList(REGISTRATION, CAREER, MAX_AI_OPPONENTS, MAX_SECONDS_IDLE,
            BATCH_CONCURRENCY, BATCH_GAMES_PER_DAY, ARCHMAGE_DAILY_CENTS, PROGRESS_XP_PERCENT, PROGRESS_COIN_PERCENT,
            DISCORD_TABLES, DISCORD_EVENTS, DISCORD_MATCHES, DISCORD_STAFF, SUPPORT_ENABLED, SUPPORT_GOAL_CENTS, SUPPORT_KOFI_URL,
            SUPPORT_SPONSORS_URL, SUPPORT_COSTS, SUPPORT_DISCORD_AMOUNTS));

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
        String raw = value == null ? "" : value.trim();
        String text = raw.toLowerCase(Locale.ROOT);
        if (!NAMES.contains(name)) {
            throw new IllegalArgumentException("There is no setting named " + name);
        }
        if (text.isEmpty()) {
            return null;
        }
        switch (name) {
            case SUPPORT_KOFI_URL:
            case SUPPORT_SPONSORS_URL:
                if (raw.length() <= MAX_URL && raw.matches("https://[^\\s\"'<>]+")) {
                    return raw;
                }
                throw new IllegalArgumentException("The page is a https:// address, " + MAX_URL + " characters at most");
            case SUPPORT_COSTS:
                if (raw.length() <= MAX_COSTS) {
                    return raw;
                }
                throw new IllegalArgumentException("What it pays for is " + MAX_COSTS + " characters at most");
            case SUPPORT_GOAL_CENTS:
                return String.valueOf(number(text, 0, 10_000_000, "The goal is 0 to 10000000 cents"));
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
            case SUPPORT_ENABLED:
            case SUPPORT_DISCORD_AMOUNTS:
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
            case ARCHMAGE_DAILY_CENTS:
                return String.valueOf(number(text, 0, 100000, "The Archmage's daily cap is 0 to 100000 cents"));
            case PROGRESS_XP_PERCENT:
            case PROGRESS_COIN_PERCENT:
                return String.valueOf(number(text, 0, 500, "Progression pay is 0 to 500 percent"));
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

    /** the Archmage AI's daily spending cap in cents; 0 (the default) keeps it off */
    public int archmageDailyCents() {
        String stored = stored(ARCHMAGE_DAILY_CENTS);
        return stored != null ? Integer.parseInt(stored) : 0;
    }

    /** XP progression pays outside Career tables, in percent; 100 by default */
    public int progressXpPercent() {
        String stored = stored(PROGRESS_XP_PERCENT);
        return stored != null ? Integer.parseInt(stored) : 100;
    }

    /** coins progression pays (the daily wins ladder), in percent; 100 by default */
    public int progressCoinPercent() {
        String stored = stored(PROGRESS_COIN_PERCENT);
        return stored != null ? Integer.parseInt(stored) : 100;
    }

    public int batchGamesPerDay() {
        String stored = stored(BATCH_GAMES_PER_DAY);
        return stored != null ? Integer.parseInt(stored) : BATCH_GAMES_PER_DAY_DEFAULT;
    }

    public int maxSecondsIdle(int fromConfig) {
        String stored = stored(MAX_SECONDS_IDLE);
        return stored != null ? Integer.parseInt(stored) : fromConfig;
    }

    // --- support for the server

    /** the support UI is on (it still needs a Ko-fi page to show) */
    public boolean supportOn() {
        return !"off".equals(stored(SUPPORT_ENABLED));
    }

    public long supportGoalCents() {
        String stored = stored(SUPPORT_GOAL_CENTS);
        return stored != null ? Long.parseLong(stored) : DEFAULT_GOAL_CENTS;
    }

    /** the Ko-fi page, or empty */
    public String supportKofiUrl() {
        String stored = stored(SUPPORT_KOFI_URL);
        return stored == null ? "" : stored;
    }

    /** the GitHub Sponsors page, or empty */
    public String supportSponsorsUrl() {
        String stored = stored(SUPPORT_SPONSORS_URL);
        return stored == null ? "" : stored;
    }

    public String supportCosts() {
        String stored = stored(SUPPORT_COSTS);
        return stored == null ? DEFAULT_COSTS : stored;
    }

    public boolean supportDiscordAmounts() {
        return "on".equals(stored(SUPPORT_DISCORD_AMOUNTS));
    }
}
