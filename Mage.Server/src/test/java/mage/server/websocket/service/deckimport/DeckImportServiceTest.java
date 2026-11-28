package mage.server.websocket.service.deckimport;

import org.junit.Assert;
import org.junit.Test;

import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.HashMap;
import java.util.Set;
import java.util.concurrent.atomic.AtomicLong;

/**
 * The deck import service's rules: which links it reads, the cache, the per-session limit and site health.
 */
public class DeckImportServiceTest {

    /** Serves recorded responses by address, and counts requests. */
    private static final class FakeFetcher implements Fetcher {
        final Map<String, String> bodies = new HashMap<>();
        final Map<String, DeckImportException.Reason> failures = new HashMap<>();
        final List<String> requested = new ArrayList<>();

        @Override
        public String get(URI uri, String accept) throws DeckImportException {
            requested.add(uri.toString());
            DeckImportException.Reason failure = failures.get(uri.toString());
            if (failure != null) {
                throw new DeckImportException(failure, "fake failure");
            }
            String body = bodies.get(uri.toString());
            if (body == null) {
                throw new DeckImportException(DeckImportException.Reason.NOT_FOUND, "no fixture for " + uri);
            }
            return body;
        }
    }

    private final AtomicLong now = new AtomicLong(1_000_000L);
    private final FakeFetcher fetcher = new FakeFetcher();
    private final DeckImportService service = new DeckImportService(DeckImportService.defaultSources(), fetcher, now::get);

    private static final String ARCHIDEKT_API = "https://archidekt.com/api/decks/3872725/";

    {
        fetcher.bodies.put(ARCHIDEKT_API, DeckImportSourcesTest.fixture("archidekt/modern-sideboard-maybeboard-3872725.json"));
        fetcher.bodies.put("https://mtgtop8.com/dec?d=896184", DeckImportSourcesTest.fixture("mtgtop8/896184.dec", StandardCharsets.ISO_8859_1));
        fetcher.bodies.put("https://mtgtop8.com/event?e=91675&d=896184&f=MO", DeckImportSourcesTest.fixture("mtgtop8/event-91675-896184.html", StandardCharsets.ISO_8859_1));
    }

    private DeckImportException.Reason failureOf(String link, String session) {
        try {
            service.importFromUrl(link, session);
        } catch (DeckImportException e) {
            return e.getReason();
        }
        Assert.fail("expected a DeckImportException for " + link);
        return null;
    }

    @Test
    public void readsADeckLinkThroughItsSource() throws Exception {
        ImportedDeck deck = service.importFromUrl("https://archidekt.com/decks/3872725/murktide?utm_source=x", "s1");
        Assert.assertEquals("Murktide", deck.getName());
        Assert.assertEquals("archidekt", deck.getSite());
        Assert.assertEquals("3872725", deck.getRemoteId());
        Assert.assertEquals(java.util.Collections.singletonList(ARCHIDEKT_API), fetcher.requested);
    }

    @Test
    public void acceptsLinksWithoutAScheme() throws Exception {
        Assert.assertEquals("5c Domain Aggro", service.importFromUrl("mtgtop8.com/event?e=91675&d=896184&f=MO", "s1").getName());
    }

    @Test
    public void refusesLinksItDoesNotRead() {
        Assert.assertEquals(DeckImportException.Reason.UNSUPPORTED, failureOf("https://www.moxfield.com/decks/abcdefgh", "s1"));
        Assert.assertEquals(DeckImportException.Reason.UNSUPPORTED, failureOf("https://example.com/decks/1", "s1"));
        Assert.assertEquals(DeckImportException.Reason.UNSUPPORTED, failureOf("https://archidekt.com/search/decks", "s1"));
        Assert.assertEquals(DeckImportException.Reason.UNSUPPORTED, failureOf("not a link at all ::", "s1"));
        Assert.assertEquals(DeckImportException.Reason.UNSUPPORTED, failureOf(null, "s1"));
        Assert.assertTrue("nothing was fetched", fetcher.requested.isEmpty());
    }

    @Test
    public void cachesForFiveMinutes() throws Exception {
        service.importFromUrl("https://archidekt.com/decks/3872725", "s1");
        service.importFromUrl("https://www.archidekt.com/decks/3872725/other-slug", "s2");
        Assert.assertEquals(1, fetcher.requested.size());
        now.addAndGet(DeckImportService.CACHE_MILLIS);
        service.importFromUrl("https://archidekt.com/decks/3872725", "s1");
        Assert.assertEquals(2, fetcher.requested.size());
    }

    @Test
    public void limitsImportsPerSessionPerMinute() throws Exception {
        for (int i = 0; i < DeckImportService.IMPORTS_PER_MINUTE; i++) {
            failureOf("https://archidekt.com/decks/" + (100 + i), "busy");
        }
        Assert.assertEquals(DeckImportException.Reason.RATE_LIMITED, failureOf("https://archidekt.com/decks/200", "busy"));
        Assert.assertEquals("other sessions aren't affected", DeckImportException.Reason.NOT_FOUND, failureOf("https://archidekt.com/decks/200", "calm"));
        now.addAndGet(60_000L);
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, failureOf("https://archidekt.com/decks/200", "busy"));
    }

    @Test
    public void sweepsIdleSessionKeysFromRateLimitMap() {
        failureOf("https://archidekt.com/decks/301", "old-1");
        failureOf("https://archidekt.com/decks/302", "old-2");
        Assert.assertEquals(2, service.trackedSessionCount());
        now.addAndGet(DeckImportService.RATE_WINDOW_MILLIS);
        failureOf("https://archidekt.com/decks/303", "fresh");
        Assert.assertEquals("expired session keys are swept", 1, service.trackedSessionCount());
    }

    @Test
    public void cachedDecksDontCountTowardTheLimit() throws Exception {
        for (int i = 0; i < DeckImportService.IMPORTS_PER_MINUTE + 5; i++) {
            service.importFromUrl("https://archidekt.com/decks/3872725", "s1");
        }
    }

    @Test
    public void marksASiteDegradedAfterRepeatedBreakage() throws Exception {
        for (int i = 0; i < DeckImportService.FAILURES_TO_DEGRADE; i++) {
            String api = "https://archidekt.com/api/decks/" + (500 + i) + "/";
            fetcher.failures.put(api, DeckImportException.Reason.BLOCKED);
            failureOf("https://archidekt.com/decks/" + (500 + i), "s" + i);
        }
        Assert.assertEquals("degraded", statusOf("archidekt"));
        Assert.assertEquals("ok", statusOf("mtgtop8"));
        now.addAndGet(DeckImportService.DEGRADED_MILLIS);
        Assert.assertEquals("ok", statusOf("archidekt"));
    }

    @Test
    public void aSuccessClearsTheDegradedMark() throws Exception {
        for (int i = 0; i < DeckImportService.FAILURES_TO_DEGRADE; i++) {
            fetcher.bodies.put("https://archidekt.com/api/decks/" + (700 + i) + "/", "<html>new site</html>");
            Assert.assertEquals(DeckImportException.Reason.SITE_CHANGED, failureOf("https://archidekt.com/decks/" + (700 + i), "s" + i));
        }
        Assert.assertEquals("degraded", statusOf("archidekt"));
        service.importFromUrl("https://archidekt.com/decks/3872725", "s9");
        Assert.assertEquals("ok", statusOf("archidekt"));
    }

    @Test
    public void missingDecksDontDegradeASite() {
        for (int i = 0; i < 5; i++) {
            failureOf("https://archidekt.com/decks/" + (900 + i), "s" + i);
        }
        Assert.assertEquals("ok", statusOf("archidekt"));
    }

    @Test
    public void listsEverySite() {
        Set<String> sites = new HashSet<>();
        for (DeckImportService.DeckSourceStatus status : service.sourceStatuses()) {
            sites.add(status.getSite());
        }
        Assert.assertEquals(new HashSet<>(java.util.Arrays.asList("archidekt", "tcgplayer", "mtgtop8", "scryfall", "manabox")), sites);
    }

    private String statusOf(String site) {
        return service.sourceStatuses().stream().filter(status -> status.getSite().equals(site)).findFirst().get().getStatus();
    }

    // the fetcher's allowlist

    @Test
    public void fetchesOnlyAllowlistedHttpsHosts() {
        Set<String> hosts = DeckImportService.allowedHosts(DeckImportService.defaultSources());
        Assert.assertTrue(HttpFetcher.allowed(URI.create("https://archidekt.com/api/decks/1/"), hosts));
        Assert.assertTrue(HttpFetcher.allowed(URI.create("https://api.scryfall.com/decks/x/export/json"), hosts));
        Assert.assertTrue(HttpFetcher.allowed(URI.create("https://cloud.manabox.app:443/decks/x"), hosts));
        Assert.assertFalse("http", HttpFetcher.allowed(URI.create("http://archidekt.com/api/decks/1/"), hosts));
        Assert.assertFalse("port", HttpFetcher.allowed(URI.create("https://archidekt.com:8443/api/decks/1/"), hosts));
        Assert.assertFalse("user info", HttpFetcher.allowed(URI.create("https://user@archidekt.com/api/decks/1/"), hosts));
        Assert.assertFalse("IP literal", HttpFetcher.allowed(URI.create("https://127.0.0.1/"), hosts));
        Assert.assertFalse("internal host", HttpFetcher.allowed(URI.create("https://localhost/"), hosts));
        Assert.assertFalse("lookalike", HttpFetcher.allowed(URI.create("https://archidekt.com.evil.example/"), hosts));
        Assert.assertFalse("blocked site", HttpFetcher.allowed(URI.create("https://api2.moxfield.com/v3/decks/all/x"), hosts));
    }

    @Test
    public void mapsHttpFailuresToReasons() {
        URI uri = URI.create("https://archidekt.com/api/decks/1/");
        Assert.assertEquals(DeckImportException.Reason.BLOCKED, HttpFetcher.failure(403, "<title>Just a moment...</title>", uri).getReason());
        Assert.assertEquals(DeckImportException.Reason.PRIVATE, HttpFetcher.failure(403, "{\"error\":\"private\"}", uri).getReason());
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, HttpFetcher.failure(404, "", uri).getReason());
        Assert.assertEquals(DeckImportException.Reason.RATE_LIMITED, HttpFetcher.failure(429, "", uri).getReason());
        Assert.assertEquals(DeckImportException.Reason.UNAVAILABLE, HttpFetcher.failure(503, "", uri).getReason());
        Assert.assertEquals(DeckImportException.Reason.SITE_CHANGED, HttpFetcher.failure(400, "", uri).getReason());
    }
}
