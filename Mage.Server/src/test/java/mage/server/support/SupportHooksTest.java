package mage.server.support;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicLong;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * The webhook listener over real HTTP, against a stand-in ledger: Ko-fi's form posts and their token, repeats, GitHub's
 * signed events, the body and rate limits, and the public status with its cache and CORS headers.
 */
public class SupportHooksTest {

    private static final String TOKEN = "tok-123";
    private static final String SECRET = "gh-s3cret";

    private final List<KofiPayload> kofi = new ArrayList<>();
    private final List<GithubSponsorsEvent> sponsorships = new ArrayList<>();
    private final Map<String, Long> seen = new HashMap<>();
    private final AtomicLong now = new AtomicLong(SupportTest.OCT_13);
    private String address = "203.0.113.5";
    private HttpServer server;

    @BeforeEach
    void listen() throws IOException {
        server = SupportHooks.listen(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), TOKEN, SECRET, new SupportHooks.Receiver() {
            @Override
            public Support.Recorded kofi(KofiPayload payment, long at) {
                Payment recorded = new Payment();
                recorded.providerId = payment.transactionId;
                boolean duplicate = seen.containsKey(payment.transactionId);
                if (!duplicate) {
                    seen.put(payment.transactionId, at);
                    kofi.add(payment);
                }
                return new Support.Recorded(recorded, duplicate);
            }

            @Override
            public void sponsorship(GithubSponsorsEvent event, long at) {
                sponsorships.add(event);
            }

            @Override
            public Support.SupportStatus status(long at) {
                Support.SupportStatus status = new Support.SupportStatus();
                status.enabled = true;
                status.goalCents = 4000;
                status.monthCents = 1500;
                status.monthLabel = "2026-10";
                status.publicSupporters = Collections.singletonList("Quinn");
                return status;
            }
        }, exchange -> address, now::get);
    }

    @AfterEach
    void stop() {
        server.stop(0);
    }

    @Test
    void kofiPaymentsAreRecordedOnceEach() throws Exception {
        String body = SupportTest.form(SupportTest.kofiJson("Donation", "tx-1", "3.00", "jo@example.test", "hi", true, false, false, null));
        Answer first = post("/hooks/kofi", body, "application/x-www-form-urlencoded", null);
        assertThat(first.status).isEqualTo(200);
        assertThat(first.body).contains("\"duplicate\":false");
        Answer again = post("/hooks/kofi", body, "application/x-www-form-urlencoded", null);
        assertThat(again.status).as("Ko-fi retries: a repeat is fine").isEqualTo(200);
        assertThat(again.body).contains("\"duplicate\":true");
        assertThat(kofi).hasSize(1);

        String renewal = SupportTest.form(SupportTest.kofiJson("Subscription", "tx-2", "5.00", "jo@example.test", null, false, true, false, "Patron"));
        assertThat(post("/hooks/kofi", renewal, "application/x-www-form-urlencoded", null).status).isEqualTo(200);
        String shop = SupportTest.form(SupportTest.kofiJson("Shop Order", "tx-3", "9.00", "jo@example.test", null, false, false, false, null));
        assertThat(post("/hooks/kofi", shop, "application/x-www-form-urlencoded", null).status).isEqualTo(200);
        assertThat(kofi).extracting(KofiPayload::kind).containsExactly("donation", "membership", "shop");
    }

    @Test
    void aWrongTokenIsRefused() throws Exception {
        String body = SupportTest.form(SupportTest.kofiJson("Donation", "tx-9", "3.00", "jo@example.test", "hi", true, false, false, null)
                .replace(TOKEN, "guess"));
        assertThat(post("/hooks/kofi", body, "application/x-www-form-urlencoded", null).status).isEqualTo(401);
        assertThat(kofi).isEmpty();
    }

    @Test
    void malformedAndOversizedBodiesAreRefused() throws Exception {
        assertThat(post("/hooks/kofi", "data=%7Bbroken", "application/x-www-form-urlencoded", null).status).isEqualTo(400);
        assertThat(post("/hooks/kofi", "hello", "text/plain", null).status).isEqualTo(400);
        StringBuilder big = new StringBuilder("data=");
        while (big.length() <= SupportHooks.MAX_BODY) {
            big.append("aaaaaaaaaaaaaaaa");
        }
        assertThat(post("/hooks/kofi", big.toString(), "application/x-www-form-urlencoded", null).status).isEqualTo(413);
        assertThat(request("GET", "/hooks/kofi").status).isEqualTo(405);
        assertThat(request("POST", "/hooks/elsewhere").status).isEqualTo(404);
        assertThat(kofi).isEmpty();
    }

    @Test
    void eachAddressHasARateLimit() throws Exception {
        for (int i = 0; i < SupportHooks.RATE_LIMIT; i++) {
            assertThat(post("/hooks/kofi", "x", "text/plain", null).status).isEqualTo(400);
        }
        assertThat(post("/hooks/kofi", "x", "text/plain", null).status).isEqualTo(429);
        address = "198.51.100.7";
        assertThat(post("/hooks/kofi", "x", "text/plain", null).status).as("another address").isEqualTo(400);
        now.addAndGet(SupportHooks.RATE_WINDOW_MS);
        address = "203.0.113.5";
        assertThat(post("/hooks/kofi", "x", "text/plain", null).status).as("a minute later").isEqualTo(400);
    }

    @Test
    void githubEventsMustBeSigned() throws Exception {
        String body = GithubSponsorsTest.event("created", 7, 500, false, "");
        Map<String, String> headers = new HashMap<>();
        headers.put("X-GitHub-Event", "sponsorship");
        headers.put("X-Hub-Signature-256", GithubSponsorsTest.sign(SECRET, body));
        assertThat(post("/hooks/github-sponsors", body, "application/json", headers).status).isEqualTo(200);
        assertThat(sponsorships).extracting(e -> e.action).containsExactly("created");

        headers.put("X-Hub-Signature-256", GithubSponsorsTest.sign("wrong", body));
        assertThat(post("/hooks/github-sponsors", body, "application/json", headers).status).isEqualTo(401);
        assertThat(sponsorships).hasSize(1);

        String ping = "{\"zen\":\"Keep it logically awesome.\",\"hook_id\":42}";
        headers.put("X-GitHub-Event", "ping");
        headers.put("X-Hub-Signature-256", GithubSponsorsTest.sign(SECRET, ping));
        Answer pong = post("/hooks/github-sponsors", ping, "application/json", headers);
        assertThat(pong.status).isEqualTo(200);
        assertThat(pong.body).contains("pong");
    }

    @Test
    void thePublicStatusIsCachedAndOpenToAnyOrigin() throws Exception {
        HttpURLConnection connection = open("GET", "/public/support.json");
        assertThat(connection.getResponseCode()).isEqualTo(200);
        assertThat(connection.getHeaderField("Access-Control-Allow-Origin")).isEqualTo("*");
        assertThat(connection.getHeaderField("Cache-Control")).isEqualTo("public, max-age=300");
        String body = read(connection);
        assertThat(body).contains("\"monthCents\":1500").contains("Quinn").doesNotContain("\"you\"");
        assertThat(request("POST", "/public/support.json").status).isEqualTo(405);
        assertThat(request("OPTIONS", "/public/support.json").status).isEqualTo(204);
        assertThat(request("GET", "/public/other.json").status).isEqualTo(404);
    }

    @Test
    void secretsAreComparedWhole() {
        assertThat(SupportHooks.sameSecret("tok-123", "tok-123")).isTrue();
        assertThat(SupportHooks.sameSecret("tok-123", "tok-12")).isFalse();
        assertThat(SupportHooks.sameSecret("tok-123", null)).isFalse();
        assertThat(SupportHooks.sameSecret("", "")).as("an unset token matches nothing").isFalse();
    }

    // ---- HTTP

    private static final class Answer {
        final int status;
        final String body;

        Answer(int status, String body) {
            this.status = status;
            this.body = body;
        }
    }

    private HttpURLConnection open(String method, String path) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL("http://127.0.0.1:" + server.getAddress().getPort() + path).openConnection();
        connection.setRequestMethod(method);
        return connection;
    }

    private Answer request(String method, String path) throws IOException {
        HttpURLConnection connection = open(method, path);
        if ("POST".equals(method)) {
            connection.setDoOutput(true);
            connection.getOutputStream().close();
        }
        return new Answer(connection.getResponseCode(), read(connection));
    }

    private Answer post(String path, String body, String type, Map<String, String> headers) throws IOException {
        HttpURLConnection connection = open("POST", path);
        connection.setDoOutput(true);
        connection.setRequestProperty("Content-Type", type);
        if (headers != null) {
            headers.forEach(connection::setRequestProperty);
        }
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        connection.setFixedLengthStreamingMode(bytes.length);
        try (OutputStream out = connection.getOutputStream()) {
            out.write(bytes);
        } catch (IOException e) {
            // the server may answer an oversized body before reading it all
        }
        return new Answer(connection.getResponseCode(), read(connection));
    }

    private static String read(HttpURLConnection connection) throws IOException {
        InputStream in = connection.getResponseCode() >= 400 ? connection.getErrorStream() : connection.getInputStream();
        if (in == null) {
            return "";
        }
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] chunk = new byte[4096];
        int read;
        while ((read = in.read(chunk)) > 0) {
            out.write(chunk, 0, read);
        }
        return new String(out.toByteArray(), StandardCharsets.UTF_8);
    }
}
