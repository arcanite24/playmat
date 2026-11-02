package mage.server.admin;

import com.google.gson.JsonObject;
import com.nimbusds.jose.JOSEException;
import com.nimbusds.jose.JWSAlgorithm;
import com.nimbusds.jose.JWSHeader;
import com.nimbusds.jose.crypto.RSASSASigner;
import com.nimbusds.jose.jwk.JWKSet;
import com.nimbusds.jose.jwk.KeyUse;
import com.nimbusds.jose.jwk.RSAKey;
import com.nimbusds.jwt.JWTClaimsSet;
import com.nimbusds.jwt.SignedJWT;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.interfaces.RSAPublicKey;
import java.util.ArrayList;
import java.util.Date;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * The identity provider client: its discovery document (fetched at most once a day, relative endpoints
 * resolved against it), the authorization URL it builds, and the code exchange it does with the provider.
 */
public class OidcAuthTest {

    private static final String DISCOVERY_URL = "https://id.example.com/.well-known/openid-configuration";
    private static final String ISSUER = "https://id.example.com/";
    private static final String CLIENT_ID = "playmat";
    private static final String CLIENT_SECRET = "s3cret";
    private static final String PUBLIC_URL = "https://play.example.test/";

    /** shaped like Pocket ID's: relative endpoints, and it promises RS256 */
    private static final String DISCOVERY_JSON =
            "{\"issuer\": \"https://id.example.com/\","
                    + "\"authorization_endpoint\": \"/authorize\","
                    + "\"token_endpoint\": \"/api/oidc/token\","
                    + "\"jwks_uri\": \"/.well-known/jwks.json\","
                    + "\"userinfo_endpoint\": \"/api/oidc/userinfo\","
                    + "\"id_token_signing_alg_values_supported\": [\"RS256\"]}";

    private long now = 1_000_000;
    private final List<String> requests = new ArrayList<>();
    private final List<String> formBodies = new ArrayList<>();
    private final List<String> bearerTokens = new ArrayList<>();
    private String discoveryJson = DISCOVERY_JSON;
    private boolean discoveryUp = true;
    private boolean userinfoUp = true;
    private String jwksJson;
    private String tokenJson = "{\"id_token\": \"never served\"}";
    private String userinfoJson = "{\"email\": \"octo@example.test\", \"email_verified\": true}";
    private String sub = "sub-octo";
    private String email = "octo@example.test";
    private Boolean emailVerified = Boolean.TRUE;
    private String preferredUsername = "OctoCat";
    private String name = null;
    private KeyPair rsa;
    private OidcAuth oidc;

    @BeforeEach
    void setUp() throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        rsa = generator.generateKeyPair();
        jwksJson = new JWKSet(new RSAKey.Builder((RSAPublicKey) rsa.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.RS256).keyID("k1").build()).toString();
        oidc = new OidcAuth(DISCOVERY_URL, CLIENT_ID, CLIENT_SECRET, "", PUBLIC_URL, false, (method, url, token, form) -> {
            requests.add(method + " " + url);
            if (form != null) {
                formBodies.add(form);
            }
            if (token != null) {
                bearerTokens.add(token);
            }
            if (url.endsWith("/.well-known/openid-configuration")) {
                if (!discoveryUp) {
                    throw new IOException("connection refused");
                }
                return discoveryJson;
            }
            if (url.endsWith("/.well-known/jwks.json")) {
                return jwksJson;
            }
            if (url.endsWith("/api/oidc/token")) {
                return tokenJson;
            }
            if (url.endsWith("/api/oidc/userinfo")) {
                if (!userinfoUp) {
                    throw new IOException("connection refused");
                }
                return userinfoJson;
            }
            throw new IOException("unexpected " + url);
        }, () -> now);
        OidcAuth.use(oidc);
    }

    @AfterEach
    void tearDown() {
        OidcAuth.use(null);
    }

    private String query(String url, String key) {
        Matcher matcher = Pattern.compile(key + "=([^&]+)").matcher(url);
        assertThat(matcher.find()).isTrue();
        return matcher.group(1);
    }

    private int hits(String suffix) {
        int count = 0;
        for (String request : requests) {
            if (request.endsWith(suffix)) {
                count++;
            }
        }
        return count;
    }

    private String tokenResponse(String nonce) {
        try {
            JWTClaimsSet.Builder builder = new JWTClaimsSet.Builder()
                    .issuer(ISSUER)
                    .audience(CLIENT_ID)
                    .subject(sub)
                    .claim("nonce", nonce)
                    .issueTime(new Date(now))
                    .expirationTime(new Date(now + 300_000));
            if (email != null) {
                builder.claim("email", email);
            }
            if (emailVerified != null) {
                builder.claim("email_verified", emailVerified);
            }
            if (preferredUsername != null) {
                builder.claim("preferred_username", preferredUsername);
            }
            if (name != null) {
                builder.claim("name", name);
            }
            SignedJWT jwt = new SignedJWT(new JWSHeader.Builder(JWSAlgorithm.RS256).keyID("k1").build(), builder.build());
            jwt.sign(new RSASSASigner(rsa.getPrivate()));
            JsonObject response = new JsonObject();
            response.addProperty("access_token", "at-1");
            response.addProperty("token_type", "Bearer");
            response.addProperty("id_token", jwt.serialize());
            return response.toString();
        } catch (JOSEException e) {
            throw new IllegalStateException(e);
        }
    }

    @Test
    void startBuildsTheAuthorizationUrl() {
        oidc.fetchDiscovery();

        String url = oidc.start();

        assertThat(url).startsWith("https://id.example.com/authorize?client_id=" + CLIENT_ID)
                .contains("redirect_uri=https%3A%2F%2Fplay.example.test%2Fauth%2Foidc")
                .contains("scope=openid%20profile%20email")
                .contains("response_type=code")
                .contains("state=")
                .contains("nonce=");
        assertThat(oidc.issuer()).as("the document's relative endpoints were resolved against its URL").isEqualTo(ISSUER);
    }

    @Test
    void unconfiguredIsRefusedWithoutCallingTheProvider() {
        OidcAuth off = new OidcAuth("", "", "", "", "", false, (method, url, token, form) -> {
            throw new IOException("the provider should never be called");
        }, () -> now);

        assertThat(off.isEnabled()).isFalse();
        assertThat(off.label()).as("the default name").isEqualTo("OIDC");
        assertThat(off.isOidcOnly()).isFalse();
        assertThatThrownBy(off::start).hasMessageContaining("isn't set up");
        off.fetchDiscovery(); // a no-op when unconfigured
        assertThat(requests).isEmpty();
    }

    @Test
    void anUnreachableProviderDefersSignInUntilItRecovers() {
        discoveryUp = false;
        oidc.fetchDiscovery();

        assertThat(oidc.discoveryProblem()).isNotNull();
        assertThatThrownBy(oidc::start).hasMessageContaining("can't be reached");

        discoveryUp = true;
        oidc.fetchDiscovery();
        assertThat(oidc.discoveryProblem()).isNull();
        assertThat(oidc.start()).contains("id.example.com/authorize");
        assertThat(hits("/.well-known/openid-configuration")).as("the boot fetch, the click's probe, the recovery fetch").isEqualTo(3);
    }

    @Test
    void aProviderThatDiesAfterDiscoveryRefusesTheButtonUntilItRecovers() {
        oidc.fetchDiscovery();
        assertThat(oidc.start()).contains("id.example.com/authorize"); // healthy: the document is fresh, no probe

        discoveryUp = false;
        now += OidcAuth.DISCOVERY_RETRY + 1; // the cached document is past the liveness window
        assertThatThrownBy(oidc::start).hasMessageContaining("can't be reached");
        int probes = hits("/.well-known/openid-configuration");
        assertThatThrownBy(oidc::start).hasMessageContaining("can't be reached");
        assertThat(hits("/.well-known/openid-configuration")).as("one probe per click, at the player's pace").isEqualTo(probes + 1);

        discoveryUp = true;
        now += 1; // the document is still stale, so the click probes again
        assertThat(oidc.start()).as("the recovered provider signs in again").contains("id.example.com/authorize");
    }

    @Test
    void aFreshCachedDocumentYieldsAUrlWithinTheLivenessWindow() {
        oidc.fetchDiscovery();
        discoveryUp = false;
        now += OidcAuth.DISCOVERY_RETRY - 60_000; // minutes stale, inside the window

        // the provider may have died in the window: the URL stands, and the sign-in is caught at the token exchange
        assertThat(oidc.start()).contains("id.example.com/authorize");
    }

    @Test
    void theDiscoveryDocumentIsFreshForADay() {
        oidc.fetchDiscovery();
        oidc.fetchDiscovery();
        assertThat(hits("/.well-known/openid-configuration")).as("a fresh document is not re-fetched").isEqualTo(1);

        now += OidcAuth.DISCOVERY_TTL + 1;
        oidc.fetchDiscovery();
        assertThat(hits("/.well-known/openid-configuration")).isEqualTo(2);
    }

    @Test
    void identifyExchangesTheCodeAndReadsTheClaims() throws Exception {
        oidc.fetchDiscovery();
        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));

        OidcAuth.Identity who = oidc.identify(query(url, "state"), "the-code");

        assertThat(who.sub).isEqualTo("sub-octo");
        assertThat(who.email).isEqualTo("octo@example.test");
        assertThat(who.emailVerified).isTrue();
        assertThat(who.displayName).isEqualTo("OctoCat");
        assertThat(requests).contains("POST https://id.example.com/api/oidc/token");
        assertThat(formBodies.get(0))
                .startsWith("grant_type=authorization_code")
                .contains("code=the-code")
                .contains("client_id=" + CLIENT_ID)
                .contains("client_secret=" + CLIENT_SECRET)
                .contains("redirect_uri=https%3A%2F%2Fplay.example.test%2Fauth%2Foidc");
        assertThat(String.join("\n", requests)).as("the client secret stays out of the URLs").doesNotContain(CLIENT_SECRET);
    }

    @Test
    void providerErrorsAndMissingCodesAreRefused() throws Exception {
        oidc.fetchDiscovery();

        tokenJson = "{\"error\": \"invalid_grant\"}";
        assertThatThrownBy(() -> oidc.identify(state(), "the-code")).hasMessageContaining("refused");

        tokenJson = "{\"access_token\": \"at\"}"; // no id_token
        assertThatThrownBy(() -> oidc.identify(state(), "the-code")).hasMessageContaining("refused");

        assertThatThrownBy(() -> oidc.identify(state(), "")).hasMessageContaining("didn't send a sign-in code");
    }

    @Test
    void aStateIsUsedOnceAndThenExpires() throws Exception {
        oidc.fetchDiscovery();
        String url = oidc.start();
        String state = query(url, "state");
        tokenJson = tokenResponse(query(url, "nonce"));

        oidc.identify(state, "the-code");
        assertThat(oidc.identify(state, "the-code").sub).as("the client asks again while naming the account").isEqualTo("sub-octo");
        assertThat(hits("/api/oidc/token")).as("the provider's code is one-use").isEqualTo(1);
        oidc.done(state);

        assertThatThrownBy(() -> oidc.identify(state, "the-code")).hasMessageContaining("expired");

        String lateUrl = oidc.start();
        now += OidcAuth.STATE_MILLIS + 1;
        tokenJson = tokenResponse(query(lateUrl, "nonce"));
        assertThatThrownBy(() -> oidc.identify(query(lateUrl, "state"), "the-code")).hasMessageContaining("expired");
        assertThatThrownBy(() -> oidc.identify("made-up", "the-code")).hasMessageContaining("expired");
    }

    @Test
    void tooManyOpenSignInsAtOnceAreRefused() {
        oidc.fetchDiscovery();
        for (int i = 0; i < OidcAuth.MAX_PENDING; i++) {
            oidc.start();
        }
        assertThatThrownBy(oidc::start).hasMessageContaining("Too many");
    }

    @Test
    void theUserinfoEndpointIsAskedForOnlyWhenTheTokenLacksAnEmail() throws Exception {
        oidc.fetchDiscovery();

        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce")); // the token carries the email
        oidc.identify(query(url, "state"), "the-code");
        assertThat(hits("/api/oidc/userinfo")).isZero();

        url = oidc.start();
        email = null;
        tokenJson = tokenResponse(query(url, "nonce"));
        OidcAuth.Identity who = oidc.identify(query(url, "state"), "the-code");
        assertThat(hits("/api/oidc/userinfo")).isEqualTo(1);
        assertThat(who.email).isEqualTo("octo@example.test");
        assertThat(who.emailVerified).isTrue();
        assertThat(bearerTokens).containsExactly("at-1"); // the profile request carried the access token

        url = oidc.start();
        userinfoUp = false;
        tokenJson = tokenResponse(query(url, "nonce"));
        who = oidc.identify(query(url, "state"), "the-code");
        assertThat(who.email).as("a failing profile lookup still lets the sign-in through").isNull();
        assertThat(who.displayName).isEqualTo("OctoCat");
    }

    @Test
    void theDisplayNamePrefersUsernameThenNameThenEmail() throws Exception {
        oidc.fetchDiscovery();
        userinfoJson = "{}"; // never fills the gaps below

        preferredUsername = "OctoCat";
        name = "Octavia Cat";
        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        assertThat(oidc.identify(query(url, "state"), "the-code").displayName).isEqualTo("OctoCat");

        preferredUsername = "  "; // a blank username is not one
        url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        assertThat(oidc.identify(query(url, "state"), "the-code").displayName).isEqualTo("Octavia Cat");

        preferredUsername = null;
        name = null;
        url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        assertThat(oidc.identify(query(url, "state"), "the-code").displayName).isEqualTo("octo"); // the email's local part

        email = null;
        url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        assertThat(oidc.identify(query(url, "state"), "the-code").displayName).isNull();
    }

    private String state() {
        return query(oidc.start(), "state");
    }
}
