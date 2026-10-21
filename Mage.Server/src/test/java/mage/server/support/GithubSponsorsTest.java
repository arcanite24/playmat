package mage.server.support;

import org.junit.jupiter.api.Test;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/** GitHub Sponsors' sponsorship events as GitHub sends them, and their signature. */
public class GithubSponsorsTest {

    /** A sponsorship event shaped like GitHub's (docs.github.com, webhook events, sponsorship). */
    static String event(String action, long sponsorId, int cents, boolean oneTime, String extra) {
        return "{\"action\":\"" + action + "\",\"sponsorship\":{\"node_id\":\"S_" + sponsorId + "\","
                + "\"created_at\":\"2026-10-10T10:00:00+00:00\","
                + "\"sponsorable\":{\"login\":\"arcanite24\",\"id\":1,\"node_id\":\"U_1\",\"type\":\"User\"},"
                + "\"sponsor\":{\"login\":\"sponsor" + sponsorId + "\",\"id\":" + sponsorId + ",\"node_id\":\"U_" + sponsorId + "\",\"type\":\"User\"},"
                + "\"privacy_level\":\"public\","
                + "\"tier\":{\"node_id\":\"T_" + cents + "\",\"created_at\":\"2026-01-01T00:00:00Z\",\"description\":\"Thanks!\","
                + "\"monthly_price_in_cents\":" + cents + ",\"monthly_price_in_dollars\":" + cents / 100 + ","
                + "\"name\":\"$" + cents / 100 + (oneTime ? " one time" : " a month") + "\",\"is_one_time\":" + oneTime + ",\"is_custom_amount\":false}}"
                + extra + ",\"sender\":{\"login\":\"sponsor" + sponsorId + "\",\"id\":" + sponsorId + "}}";
    }

    static String sign(String secret, String body) throws Exception {
        Mac mac = Mac.getInstance("HmacSHA256");
        mac.init(new SecretKeySpec(secret.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
        return "sha256=" + GithubSponsorsEvent.hex(mac.doFinal(body.getBytes(StandardCharsets.UTF_8)));
    }

    @Test
    void readsASponsorship() {
        GithubSponsorsEvent created = GithubSponsorsEvent.parse(event("created", 7, 500, false, ""));
        assertThat(created.action).isEqualTo("created");
        assertThat(created.nodeId).isEqualTo("S_7");
        assertThat(created.sponsorId).isEqualTo(7);
        assertThat(created.sponsorLogin).isEqualTo("sponsor7");
        assertThat(created.monthlyCents).isEqualTo(500);
        assertThat(created.isPublic).isTrue();
        assertThat(created.oneTime).isFalse();
        assertThat(created.createdAt).isEqualTo(java.time.Instant.parse("2026-10-10T10:00:00Z").toEpochMilli());

        GithubSponsorsEvent pending = GithubSponsorsEvent.parse(event("pending_cancellation", 7, 500, false,
                ",\"effective_date\":\"2026-11-10T00:00:00+00:00\""));
        assertThat(pending.effectiveAt).isEqualTo(java.time.Instant.parse("2026-11-10T00:00:00Z").toEpochMilli());
        assertThat(GithubSponsorsEvent.parse(event("created", 8, 2500, true, "")).oneTime).isTrue();

        assertThatThrownBy(() -> GithubSponsorsEvent.parse("{\"zen\":\"Design for failure.\",\"hook_id\":1}"))
                .isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> GithubSponsorsEvent.parse("<html>")).isInstanceOf(IllegalArgumentException.class);
    }

    @Test
    void checksTheSignature() throws Exception {
        String body = event("created", 7, 500, false, "");
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        assertThat(GithubSponsorsEvent.verify("s3cret", bytes, sign("s3cret", body))).isTrue();
        assertThat(GithubSponsorsEvent.verify("s3cret", bytes, sign("other", body))).isFalse();
        assertThat(GithubSponsorsEvent.verify("s3cret", (body + " ").getBytes(StandardCharsets.UTF_8), sign("s3cret", body))).isFalse();
        assertThat(GithubSponsorsEvent.verify("s3cret", bytes, null)).isFalse();
        assertThat(GithubSponsorsEvent.verify("s3cret", bytes, "sha1=abc")).isFalse();
        assertThat(GithubSponsorsEvent.verify("", bytes, "sha256=00")).as("no secret, no trust").isFalse();
    }
}
