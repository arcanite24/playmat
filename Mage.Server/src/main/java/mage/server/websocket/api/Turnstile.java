package mage.server.websocket.api;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;

/**
 * Web client bridge: Cloudflare Turnstile, the "are you a person" check on registration and access requests. Off
 * unless both XMAGE_TURNSTILE_SITE_KEY (shown to the web client in serverInfo) and XMAGE_TURNSTILE_SECRET are set.
 * When on, every token is checked with Cloudflare before the account is made, and anything but a clear yes refuses:
 * a missing or failed token, and Cloudflare being unreachable (logged), alike.
 */
final class Turnstile {

    private static final Logger logger = Logger.getLogger(Turnstile.class);

    static final String VERIFY_URL = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
    static final int TIMEOUT_MS = 5_000;
    static final String SITE_KEY_PROP = "xmage.turnstile.siteKey";
    static final String SITE_KEY_ENV = "XMAGE_TURNSTILE_SITE_KEY";
    static final String SECRET_PROP = "xmage.turnstile.secret";
    static final String SECRET_ENV = "XMAGE_TURNSTILE_SECRET";

    static final String MISSING = "Complete the check that you're a person first.";
    static final String FAILED = "The check that you're a person didn't pass. Try it again.";
    static final String UNREACHABLE = "The check that you're a person can't be confirmed right now. Try again in a minute.";

    /** Posts a form to a URL and returns the response body; for tests, without the network. */
    @FunctionalInterface
    interface Http {
        String post(String url, String form) throws IOException;
    }

    private final String siteKey;
    private final String secret;
    private final Http http;

    Turnstile(String siteKey, String secret, Http http) {
        this.siteKey = blank(siteKey) ? "" : siteKey.trim();
        this.secret = blank(secret) ? "" : secret.trim();
        this.http = http;
    }

    /** The server's settings: system properties first, then the environment. */
    static Turnstile fromSettings() {
        return new Turnstile(setting(SITE_KEY_PROP, SITE_KEY_ENV), setting(SECRET_PROP, SECRET_ENV), Turnstile::post);
    }

    boolean enabled() {
        return !siteKey.isEmpty() && !secret.isEmpty();
    }

    /** For the web client's widget; null when Turnstile is off. */
    String siteKey() {
        return enabled() ? siteKey : null;
    }

    /**
     * @return null when the token passes (or Turnstile is off); otherwise why not, for the player
     */
    String check(String token, String remoteIp) {
        if (!enabled()) {
            return null;
        }
        if (blank(token)) {
            return MISSING;
        }
        String form = "secret=" + encode(secret) + "&response=" + encode(token.trim())
                + (blank(remoteIp) ? "" : "&remoteip=" + encode(remoteIp.trim()));
        String body;
        try {
            body = http.post(VERIFY_URL, form);
        } catch (IOException e) {
            logger.warn("Turnstile can't be reached, refusing: " + e.getMessage());
            return UNREACHABLE;
        }
        try {
            JsonObject answer = JsonParser.parseString(body).getAsJsonObject();
            JsonElement success = answer.get("success");
            if (success != null && success.isJsonPrimitive() && success.getAsBoolean()) {
                return null;
            }
            logger.info("Turnstile refused a token: " + answer.get("error-codes"));
            return FAILED;
        } catch (RuntimeException e) {
            logger.warn("Turnstile answered something unreadable, refusing");
            return UNREACHABLE;
        }
    }

    private static String post(String url, String form) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(url).openConnection();
        try {
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setRequestMethod("POST");
            connection.setDoOutput(true);
            connection.setRequestProperty("Content-Type", "application/x-www-form-urlencoded");
            try (OutputStream out = connection.getOutputStream()) {
                out.write(form.getBytes(StandardCharsets.UTF_8));
            }
            int status = connection.getResponseCode();
            if (status != 200) {
                throw new IOException("HTTP " + status);
            }
            try (InputStream in = connection.getInputStream()) {
                ByteArrayOutputStream body = new ByteArrayOutputStream();
                byte[] buffer = new byte[4096];
                int read;
                while ((read = in.read(buffer)) != -1 && body.size() < 64 * 1024) {
                    body.write(buffer, 0, read);
                }
                return new String(body.toByteArray(), StandardCharsets.UTF_8);
            }
        } finally {
            connection.disconnect();
        }
    }

    private static String setting(String property, String environment) {
        String value = System.getProperty(property);
        return blank(value) ? System.getenv(environment) : value;
    }

    private static String encode(String text) {
        try {
            return URLEncoder.encode(text, "UTF-8");
        } catch (java.io.UnsupportedEncodingException e) {
            throw new IllegalStateException(e);
        }
    }

    private static boolean blank(String text) {
        return text == null || text.trim().isEmpty();
    }
}
