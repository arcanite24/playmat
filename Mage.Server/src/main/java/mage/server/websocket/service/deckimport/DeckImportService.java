package mage.server.websocket.service.deckimport;

import mage.utils.MageVersion;
import org.apache.log4j.Logger;

import java.net.URI;
import java.net.URISyntaxException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Deque;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.LongSupplier;

/**
 * Web client bridge: imports decks from deck websites by link (DeckImportApi).
 * <p>
 * Reads only the sites in {@link #defaultSources()}, through a fetcher limited to their hosts.
 * Results are cached for five minutes, each session may import ten decks a minute, and a site that keeps
 * failing in ways that mean it changed or blocks us is reported as degraded for half an hour,
 * so the client sends players to its copy-and-paste flow straight away.
 */
public final class DeckImportService {

    private static final Logger logger = Logger.getLogger(DeckImportService.class);

    static final long CACHE_MILLIS = 5 * 60_000L;
    static final int CACHE_SIZE = 200;
    static final int IMPORTS_PER_MINUTE = 10;
    static final long RATE_WINDOW_MILLIS = 60_000L;
    static final int FAILURES_TO_DEGRADE = 3;
    static final long DEGRADED_MILLIS = 30 * 60_000L;

    /** One site and whether its direct import works right now. */
    public static final class DeckSourceStatus {
        private final String site;
        private final String status;

        DeckSourceStatus(String site, String status) {
            this.site = site;
            this.status = status;
        }

        public String getSite() {
            return site;
        }

        /** "ok" or "degraded" */
        public String getStatus() {
            return status;
        }
    }

    private static final class Cached {
        final ImportedDeck deck;
        final long at;

        Cached(ImportedDeck deck, long at) {
            this.deck = deck;
            this.at = at;
        }
    }

    private static final class Health {
        int consecutiveFailures;
        long degradedUntil;
        int successes;
        int failures;
    }

    private final List<DeckSource> sources;
    private final Fetcher fetcher;
    private final LongSupplier clock;
    private final Map<String, Cached> cache = Collections.synchronizedMap(new LinkedHashMap<String, Cached>(16, 0.75f, true) {
        @Override
        protected boolean removeEldestEntry(Map.Entry<String, Cached> eldest) {
            return size() > CACHE_SIZE;
        }
    });
    private final Map<String, Health> health = new HashMap<>();
    private final Map<String, Deque<Long>> recentImports = new ConcurrentHashMap<>();

    public DeckImportService() {
        this(defaultSources(), null, System::currentTimeMillis);
    }

    DeckImportService(List<DeckSource> sources, Fetcher fetcher, LongSupplier clock) {
        this.sources = sources;
        this.fetcher = fetcher != null ? fetcher : new HttpFetcher(allowedHosts(sources), userAgent());
        this.clock = clock;
    }

    static List<DeckSource> defaultSources() {
        return Arrays.asList(new ArchidektSource(), new TcgplayerSource(), new MtgTop8Source(), new ScryfallSource(), new ManaboxSource());
    }

    static Set<String> allowedHosts(List<DeckSource> sources) {
        Set<String> hosts = new TreeSet<>();
        for (DeckSource source : sources) {
            hosts.addAll(source.fetchHosts());
        }
        return Collections.unmodifiableSet(hosts);
    }

    private static String userAgent() {
        return "Playmat/" + MageVersion.MAGE_VERSION_MAJOR + "." + MageVersion.MAGE_VERSION_MINOR + "." + MageVersion.MAGE_VERSION_RELEASE
                + " (XMage web client; deck import on request)";
    }

    /**
     * Reads the deck behind a deck link.
     *
     * @param sessionKey who is importing, for the per-session limit
     */
    public ImportedDeck importFromUrl(String link, String sessionKey) throws DeckImportException {
        URI url = parse(link);
        DeckSource source = sourceFor(url);
        String deckId = source == null ? null : source.deckId(url);
        if (source == null || deckId == null) {
            throw new DeckImportException(DeckImportException.Reason.UNSUPPORTED, "Not a deck link the server reads");
        }

        String cacheKey = source.id() + ":" + deckId;
        long now = clock.getAsLong();
        Cached cached = cache.get(cacheKey);
        if (cached != null && now - cached.at < CACHE_MILLIS) {
            return cached.deck;
        }
        checkRate(sessionKey, now);

        try {
            ImportedDeck deck = source.fetch(url, deckId, fetcher);
            cache.put(cacheKey, new Cached(deck, now));
            record(source.id(), null);
            return deck;
        } catch (DeckImportException e) {
            record(source.id(), e.getReason());
            throw e;
        } catch (RuntimeException e) {
            // a parser tripping over an unexpected shape is the site changing, never a server error
            record(source.id(), DeckImportException.Reason.SITE_CHANGED);
            throw new DeckImportException(DeckImportException.Reason.SITE_CHANGED, source.id() + " answered in a shape we don't read", e);
        }
    }

    /** Every site the server reads, with whether it works right now. */
    public List<DeckSourceStatus> sourceStatuses() {
        long now = clock.getAsLong();
        List<DeckSourceStatus> statuses = new ArrayList<>();
        synchronized (health) {
            for (DeckSource source : sources) {
                Health site = health.get(source.id());
                boolean degraded = site != null && site.degradedUntil > now;
                statuses.add(new DeckSourceStatus(source.id(), degraded ? "degraded" : "ok"));
            }
        }
        return statuses;
    }

    private static URI parse(String link) throws DeckImportException {
        if (link == null || link.length() > 2048) {
            throw new DeckImportException(DeckImportException.Reason.UNSUPPORTED, "Not a deck link");
        }
        String trimmed = link.trim();
        if (!trimmed.matches("(?i)^https?://.*")) {
            trimmed = "https://" + trimmed;
        }
        try {
            URI uri = new URI(trimmed);
            if (uri.getHost() == null) {
                throw new DeckImportException(DeckImportException.Reason.UNSUPPORTED, "Not a deck link");
            }
            return uri;
        } catch (URISyntaxException e) {
            throw new DeckImportException(DeckImportException.Reason.UNSUPPORTED, "Not a deck link", e);
        }
    }

    private DeckSource sourceFor(URI url) {
        String host = url.getHost().toLowerCase(Locale.ROOT);
        for (DeckSource source : sources) {
            if (source.pageHosts().contains(host)) {
                return source;
            }
        }
        return null;
    }

    private void checkRate(String sessionKey, long now) throws DeckImportException {
        pruneIdleSessions(now);
        String key = sessionKey == null ? "" : sessionKey;
        boolean[] limited = {false};
        recentImports.compute(key, (k, existing) -> {
            Deque<Long> times = existing != null ? existing : new ArrayDeque<>();
            pruneExpired(times, now);
            if (times.size() >= IMPORTS_PER_MINUTE) {
                limited[0] = true;
                return times;
            }
            times.addLast(now);
            return times;
        });
        if (limited[0]) {
            throw new DeckImportException(DeckImportException.Reason.RATE_LIMITED, "Too many deck imports in a minute");
        }
    }

    private void pruneIdleSessions(long now) {
        for (String key : recentImports.keySet()) {
            recentImports.computeIfPresent(key, (k, times) -> {
                pruneExpired(times, now);
                return times.isEmpty() ? null : times;
            });
        }
    }

    private static void pruneExpired(Deque<Long> times, long now) {
        while (!times.isEmpty() && now - times.peekFirst() >= RATE_WINDOW_MILLIS) {
            times.pollFirst();
        }
    }

    int trackedSessionCount() {
        return recentImports.size();
    }

    private void record(String site, DeckImportException.Reason failure) {
        long now = clock.getAsLong();
        int successes;
        int failures;
        synchronized (health) {
            Health entry = health.computeIfAbsent(site, key -> new Health());
            if (failure == null) {
                entry.successes++;
                entry.consecutiveFailures = 0;
                entry.degradedUntil = 0;
            } else {
                entry.failures++;
                if (failure == DeckImportException.Reason.SITE_CHANGED || failure == DeckImportException.Reason.BLOCKED) {
                    entry.consecutiveFailures++;
                    if (entry.consecutiveFailures >= FAILURES_TO_DEGRADE) {
                        entry.degradedUntil = now + DEGRADED_MILLIS;
                    }
                }
            }
            successes = entry.successes;
            failures = entry.failures;
        }
        // counts only: never deck contents, links or user names
        logger.info("Deck import from " + site + ": " + (failure == null ? "ok" : failure.wireName())
                + " (" + successes + " ok, " + failures + " failed since start)");
    }
}
