package mage.server.support;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParseException;
import com.google.gson.JsonParser;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.time.OffsetDateTime;
import java.time.format.DateTimeParseException;

/**
 * One GitHub Sponsors webhook delivery (a sponsorship event): who sponsors, at which tier, and what happened. GitHub
 * signs the body with the webhook's secret (X-Hub-Signature-256: HMAC-SHA256), checked before anything is read.
 */
public final class GithubSponsorsEvent {

    /** created, cancelled, pending_cancellation, tier_changed, pending_tier_change or edited */
    public final String action;
    public final String nodeId;
    public final long sponsorId;
    public final String sponsorLogin;
    public final boolean isPublic;
    public final long monthlyCents;
    public final String tierName;
    public final boolean oneTime;
    /** when the sponsorship began (epoch ms), 0 when GitHub didn't say */
    public final long createdAt;
    /** when a pending change takes effect (epoch ms), 0 when there is none */
    public final long effectiveAt;
    public final String json;

    private GithubSponsorsEvent(JsonObject root, String json) {
        this.json = json;
        action = string(root, "action");
        JsonObject sponsorship = object(root, "sponsorship");
        JsonObject sponsor = object(sponsorship, "sponsor");
        JsonObject tier = object(sponsorship, "tier");
        nodeId = string(sponsorship, "node_id");
        sponsorId = number(sponsor, "id");
        sponsorLogin = string(sponsor, "login");
        isPublic = "public".equals(string(sponsorship, "privacy_level"));
        monthlyCents = number(tier, "monthly_price_in_cents");
        tierName = string(tier, "name");
        JsonElement once = tier == null ? null : tier.get("is_one_time");
        oneTime = once != null && once.isJsonPrimitive() && once.getAsBoolean();
        createdAt = time(string(sponsorship, "created_at"));
        effectiveAt = time(string(root, "effective_date"));
    }

    /**
     * Reads a sponsorship event.
     *
     * @throws IllegalArgumentException when it isn't one
     */
    public static GithubSponsorsEvent parse(String json) {
        JsonElement parsed;
        try {
            parsed = JsonParser.parseString(json == null ? "" : json);
        } catch (JsonParseException e) {
            throw new IllegalArgumentException("not JSON");
        }
        if (!parsed.isJsonObject()) {
            throw new IllegalArgumentException("not a JSON object");
        }
        GithubSponsorsEvent event = new GithubSponsorsEvent(parsed.getAsJsonObject(), json);
        if (event.action == null || event.nodeId == null || event.sponsorId <= 0) {
            throw new IllegalArgumentException("not a sponsorship event");
        }
        return event;
    }

    /** Whether {@code signature} (the X-Hub-Signature-256 header) is the body's HMAC-SHA256 with the secret. */
    public static boolean verify(String secret, byte[] body, String signature) {
        if (secret == null || secret.isEmpty() || signature == null || !signature.startsWith("sha256=")) {
            return false;
        }
        try {
            Mac mac = Mac.getInstance("HmacSHA256");
            mac.init(new SecretKeySpec(secret.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
            String expected = "sha256=" + hex(mac.doFinal(body));
            return MessageDigest.isEqual(expected.getBytes(StandardCharsets.US_ASCII), signature.trim().getBytes(StandardCharsets.US_ASCII));
        } catch (GeneralSecurityException e) {
            return false;
        }
    }

    static String hex(byte[] bytes) {
        StringBuilder text = new StringBuilder(bytes.length * 2);
        for (byte b : bytes) {
            text.append(Character.forDigit((b >> 4) & 0xf, 16)).append(Character.forDigit(b & 0xf, 16));
        }
        return text.toString();
    }

    private static JsonObject object(JsonObject parent, String name) {
        JsonElement value = parent == null ? null : parent.get(name);
        return value != null && value.isJsonObject() ? value.getAsJsonObject() : null;
    }

    private static String string(JsonObject parent, String name) {
        JsonElement value = parent == null ? null : parent.get(name);
        return value != null && value.isJsonPrimitive() ? value.getAsString() : null;
    }

    private static long number(JsonObject parent, String name) {
        JsonElement value = parent == null ? null : parent.get(name);
        try {
            return value != null && value.isJsonPrimitive() ? value.getAsLong() : 0;
        } catch (NumberFormatException e) {
            return 0;
        }
    }

    private static long time(String text) {
        if (text == null) {
            return 0;
        }
        try {
            return OffsetDateTime.parse(text).toInstant().toEpochMilli();
        } catch (DateTimeParseException e) {
            return 0;
        }
    }
}
