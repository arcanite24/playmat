package mage.server.websocket.api;

import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * What stops scripts from making accounts: a per-address limit, and Cloudflare Turnstile checked without a network.
 */
public class RegistrationGuardsTest {

    private static final long MINUTE = 60_000L;
    private static final long HOUR = 60 * MINUTE;

    @Test
    void threeAnHourPerAddress() {
        AtomicLong now = new AtomicLong(0);
        RegistrationLimiter limiter = new RegistrationLimiter(now::get);
        for (int i = 0; i < RegistrationLimiter.PER_HOUR; i++) {
            assertThat(limiter.take("203.0.113.5")).isNull();
            now.addAndGet(MINUTE);
        }
        assertThat(limiter.take("203.0.113.5")).contains("last hour");
        assertThat(limiter.take("198.51.100.7")).as("another address").isNull();

        now.set(HOUR + 1);
        assertThat(limiter.take("203.0.113.5")).as("the oldest is an hour old").isNull();
    }

    @Test
    void tenADayPerAddress() {
        AtomicLong now = new AtomicLong(0);
        RegistrationLimiter limiter = new RegistrationLimiter(now::get);
        for (int i = 0; i < RegistrationLimiter.PER_DAY; i++) {
            assertThat(limiter.take("203.0.113.5")).as("account " + i).isNull();
            now.addAndGet(HOUR / 2 + 1);
            if (i % 3 == 2) {
                now.addAndGet(HOUR);
            }
        }
        now.addAndGet(2 * HOUR);
        assertThat(limiter.take("203.0.113.5")).contains("today");
        now.addAndGet(24 * HOUR);
        assertThat(limiter.take("203.0.113.5")).isNull();
    }

    @Test
    void aFailedRegistrationDoesntCount() {
        RegistrationLimiter limiter = new RegistrationLimiter(() -> 0);
        for (int i = 0; i < 10; i++) {
            assertThat(limiter.take("203.0.113.5")).isNull();
            limiter.refund("203.0.113.5");
        }
        assertThat(limiter.take("203.0.113.5")).isNull();
    }

    @Test
    void turnstileIsOffWithoutBothKeys() {
        Turnstile off = new Turnstile("site", "", (url, form) -> {
            throw new AssertionError("no call when off");
        });
        assertThat(off.enabled()).isFalse();
        assertThat(off.siteKey()).isNull();
        assertThat(off.check(null, "203.0.113.5")).isNull();
    }

    @Test
    void turnstileSendsTheSecretTokenAndAddress() {
        List<String> calls = new ArrayList<>();
        Turnstile turnstile = new Turnstile("site-key", "s3cr&t", (url, form) -> {
            calls.add(url + " " + form);
            return "{\"success\":true,\"error-codes\":[]}";
        });
        assertThat(turnstile.siteKey()).isEqualTo("site-key");
        assertThat(turnstile.check("tok en", "203.0.113.5")).isNull();
        assertThat(calls).containsExactly(Turnstile.VERIFY_URL + " secret=s3cr%26t&response=tok+en&remoteip=203.0.113.5");
    }

    @Test
    void turnstileFailsClosed() {
        Turnstile refusing = new Turnstile("k", "s", (url, form) -> "{\"success\":false,\"error-codes\":[\"invalid-input-response\"]}");
        assertThat(refusing.check("token", null)).isEqualTo(Turnstile.FAILED);
        assertThat(refusing.check(" ", null)).as("no token").isEqualTo(Turnstile.MISSING);

        Turnstile unreachable = new Turnstile("k", "s", (url, form) -> {
            throw new IOException("connect timed out");
        });
        assertThat(unreachable.check("token", null)).isEqualTo(Turnstile.UNREACHABLE);

        Turnstile garbled = new Turnstile("k", "s", (url, form) -> "<html>busy</html>");
        assertThat(garbled.check("token", null)).isEqualTo(Turnstile.UNREACHABLE);
    }
}
