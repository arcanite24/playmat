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
import mage.server.AuthorizedUserRepository;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Path;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.interfaces.RSAPublicKey;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Date;
import java.util.List;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * The end of an identity-provider sign-in: a one-use state, accounts found by link or verified email, new
 * accounts named to fit, and an allowlist server that lets only the listed in.
 */
public class OidcSignInTest {

    private static final String DISCOVERY_URL = "https://id.example.com/.well-known/openid-configuration";
    private static final String ISSUER = "https://id.example.com/";
    private static final String CLIENT_ID = "playmat";
    private static final String PUBLIC_URL = "https://play.example.test/";

    /** this server's provider has no userinfo endpoint */
    private static final String DISCOVERY_JSON =
            "{\"issuer\": \"https://id.example.com/\","
                    + "\"authorization_endpoint\": \"/authorize\","
                    + "\"token_endpoint\": \"/api/oidc/token\","
                    + "\"jwks_uri\": \"/.well-known/jwks.json\"}";

    @TempDir
    Path dir;

    private long now = 1_000_000;
    private final List<String> requests = new ArrayList<>();
    private String jwksJson;
    private String tokenJson = "{\"id_token\": \"never served\"}";
    private String sub = "sub-octo";
    private String email = "octo@example.test";
    private Boolean emailVerified = Boolean.TRUE;
    private String preferredUsername = "octo-cat-with-a-long-name";
    private String name = null;
    private Object groupClaimValue = null; // the value to embed under the token's groups claim, null for none
    private KeyPair rsa;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private OidcAuth.Http http;
    private OidcAuth oidc;

    @BeforeEach
    void setUp() throws Exception {
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:oidc-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        rsa = generator.generateKeyPair();
        jwksJson = new JWKSet(new RSAKey.Builder((RSAPublicKey) rsa.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.RS256).keyID("k1").build()).toString();
        http = (method, url, token, form) -> {
            requests.add(method + " " + url);
            if (url.endsWith("/.well-known/openid-configuration")) {
                return DISCOVERY_JSON;
            }
            if (url.endsWith("/.well-known/jwks.json")) {
                return jwksJson;
            }
            if (url.endsWith("/api/oidc/token")) {
                return tokenJson;
            }
            throw new IOException("unexpected " + url);
        };
        oidc = new OidcAuth(DISCOVERY_URL, CLIENT_ID, "s3cret", "", PUBLIC_URL, false, "", "", "", http, () -> now);
        OidcAuth.use(oidc);
        oidc.fetchDiscovery();
    }

    /** The provider with a given admin-group configuration (its config is read once at construction). */
    private OidcAuth provider(String adminGroup, String scope, String groupsClaim) {
        oidc = new OidcAuth(DISCOVERY_URL, CLIENT_ID, "s3cret", "", PUBLIC_URL, false, adminGroup, scope, groupsClaim, http, () -> now);
        OidcAuth.use(oidc);
        oidc.fetchDiscovery();
        return oidc;
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

    /** a fresh state, with the provider's token response ready for its nonce */
    private String ready() {
        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        return query(url, "state");
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
            if (groupClaimValue != null) {
                builder.claim("groups", groupClaimValue);
            }
            SignedJWT jwt = new SignedJWT(new JWSHeader.Builder(JWSAlgorithm.RS256).keyID("k1").build(), builder.build());
            jwt.sign(new RSASSASigner(rsa.getPrivate()));
            JsonObject response = new JsonObject();
            response.addProperty("access_token", "at-1");
            response.addProperty("id_token", jwt.serialize());
            return response.toString();
        } catch (JOSEException e) {
            throw new IllegalStateException(e);
        }
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

    private OidcSignIn signIn(AccessPolicy policy) {
        return new OidcSignIn(oidc, policy, accounts);
    }

    @Test
    void authorizeLinkComesBackToTheClient() {
        String url = oidc.start();

        assertThat(url).startsWith("https://id.example.com/authorize?client_id=" + CLIENT_ID)
                .contains("redirect_uri=https%3A%2F%2Fplay.example.test%2Fauth%2Foidc")
                .contains("scope=openid%20profile%20email")
                .contains("response_type=code");
    }

    @Test
    void aNewAccountGetsANameThatFits() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);

        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(result.userName).isEqualTo("octo_cat_with_");
        assertThat(result.rememberToken).isNotEmpty();
        assertThat(result.idTokenRef).as("the sign-in's ID-token ref rides the result for the sign-out's end-session hop").isNotBlank();
        assertThat(oidc.takeTokenRef(result.idTokenRef).sub).isEqualTo("sub-octo");
        assertThat(accounts.isRememberedLogin("octo_cat_with_", result.rememberToken)).isTrue();
        assertThat(store.oidcLink("sub-octo").account).isEqualTo("octo_cat_with_");
        assertThat(store.oidcLink("sub-octo").displayName).isEqualTo("octo-cat-with-a-long-name");
        assertThat(store.accountEmail("octo_cat_with_").registeredAt).isEqualTo(now);
        assertThat(store.auditLog(null, "access.oidc.register", null, 0, 10)).hasSize(1);
    }

    @Test
    void stateWorksOnceAndExpires() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        String state = ready();

        OidcSignIn.OidcSignInResult first = signIn(open).finish(true, state, "the-code", null, now);
        assertThat(first.userName).isEqualTo("octo_cat_with_");
        assertThatThrownBy(() -> signIn(open).finish(true, state, "the-code", null, now))
                .as("the sign-in is finished, so its state is spent").hasMessageContaining("expired");

        String late = ready();
        now += OidcAuth.STATE_MILLIS + 1;
        assertThatThrownBy(() -> signIn(open).finish(true, late, "the-code", null, now)).hasMessageContaining("expired");
        assertThatThrownBy(() -> signIn(open).finish(true, "made-up", "the-code", null, now)).hasMessageContaining("expired");
    }

    @Test
    void aTakenNameAsksForAnother() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test");
        String state = ready();

        OidcSignIn.OidcSignInResult ask = signIn(open).finish(true, state, "the-code", null, now);
        assertThat(ask.userName).isNull();
        assertThat(ask.suggestedName).isEqualTo("octo_cat_with_");

        assertThat(ask.resume).isNotBlank();
        assertThatThrownBy(() -> signIn(open).finish(true, state, "the-code", "octo", now))
                .as("the state from the callback URL can't name the account without the resume secret").hasMessageContaining("expired");
        assertThat(accounts.getByName("octo")).isNull();
    }

    @Test
    void theNamingRetryNeedsTheResumeSecret() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test");
        String state = ready();

        OidcSignIn.OidcSignInResult ask = signIn(open).finish(true, state, "the-code", null, now);
        OidcSignIn.OidcSignInResult named = signIn(open).finish(true, state, ask.resume, "octo", now);
        assertThat(named.userName).isEqualTo("octo");
        assertThat(hits("/api/oidc/token")).as("the provider's code is one-use").isEqualTo(1);
    }

    @Test
    void aTakenNameAsksAgainForANewPerson() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test");

        sub = "sub-newbie"; // a first-time person
        email = "newbie@example.test";
        String second = ready();
        OidcSignIn.OidcSignInResult invalid = signIn(open).finish(true, second, "the-code", "way too long a name here", now);
        assertThat(invalid.userName).isNull();
        assertThat(invalid.suggestedName).isEqualTo("octo_cat_with_");
        assertThat(invalid.nameProblem).contains("3 to 14");
    }

    @Test
    void aLinkedAccountSignsInAgain() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octavia", "secret1", "old@example.test");
        store.linkOidc("sub-octo", "octo-cat-with-a-long-name", "old@example.test", "octavia", now);

        email = "changed@example.test"; // the provider's email moved on
        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(result.userName).as("the link wins over the email").isEqualTo("octavia");
        assertThat(result.idTokenRef).isNotBlank();
        assertThat(oidc.takeTokenRef(result.idTokenRef).sub).isEqualTo("sub-octo");
        assertThat(store.oidcLink("sub-octo").linkedAt).isEqualTo(now);
        assertThat(store.accountEmail("octavia")).as("the new address is not vouched").isNull();
    }

    @Test
    void theRefComesWithTheAccountThatFinallySignsIn() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octo_cat_with_", "secret1", "someone@example.test"); // the suggested name is taken
        String state = ready();

        OidcSignIn.OidcSignInResult ask = signIn(open).finish(true, state, "the-code", null, now);
        assertThat(ask.idTokenRef).as("a naming ask has no signed-in account yet, so nothing to hand over").isNull();

        OidcSignIn.OidcSignInResult named = signIn(open).finish(true, state, ask.resume, "octo", now);
        assertThat(named.userName).isEqualTo("octo");
        assertThat(named.idTokenRef).isNotBlank();
        assertThat(oidc.takeTokenRef(named.idTokenRef).sub).isEqualTo("sub-octo");
    }

    @Test
    void aVerifiedEmailSignsInToTheExistingAccount() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octavia", "secret1", "octo@example.test");
        store.emailVerified("octavia", "octo@example.test", now); // the account confirmed the address itself

        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(result.userName).isEqualTo("octavia");
        assertThat(store.oidcLink("sub-octo").account).isEqualTo("octavia");
        assertThat(store.accountEmail("octavia").verifiedEmail).as("the provider vouches the address").isEqualTo("octo@example.test");
    }

    @Test
    void anAccountThatNeverConfirmedTheEmailIsNotTakenOver() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("squatter", "secret1", "octo@example.test"); // registered with someone else's address

        assertThatThrownBy(() -> signIn(open).finish(true, ready(), "the-code", null, now))
                .hasMessage(OidcSignIn.UNCONFIRMED_EMAIL);
        assertThat(store.oidcLink("sub-octo")).as("nothing is linked").isNull();
        assertThat(store.accountEmail("squatter")).as("nor vouched").isNull();
    }

    @Test
    void aRefusedLinkedSignInWritesNothing() throws Exception {
        AccessPolicy closed = AccessPolicy.create(store, "", "allowlist", "", now);
        accounts.add("octavia", "secret1", "octo@example.test");
        store.emailVerified("octavia", "octo@example.test", now);

        assertThatThrownBy(() -> signIn(closed).finish(true, ready(), "the-code", null, now)).hasMessage(AccessPolicy.REFUSED);
        assertThat(store.oidcLink("sub-octo")).isNull();
    }

    @Test
    void anUnverifiedEmailDoesNotMatchAnExistingAccount() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octavia", "secret1", "octo@example.test");
        emailVerified = Boolean.FALSE;

        assertThatThrownBy(() -> signIn(open).finish(true, ready(), "the-code", null, now)).hasMessageContaining("verified email");
    }

    @Test
    void anAllowlistLetsOnlyListedPeopleIn() throws Exception {
        AccessPolicy closed = AccessPolicy.create(store, "", "allowlist", "", now);

        assertThatThrownBy(() -> signIn(closed).finish(true, ready(), "the-code", null, now)).hasMessage(AccessPolicy.REFUSED);

        store.allow("octo@example.test", "friend", "owner", now);

        OidcSignIn.OidcSignInResult result = signIn(closed).finish(true, ready(), "the-code", null, now);
        assertThat(result.userName).isEqualTo("octo_cat_with_");
        assertThat(closed.allows("octo_cat_with_", "octo@example.test")).isTrue();
        assertThat(store.allowlist()).extracting(e -> e.note).contains("friend").anyMatch(note -> note.startsWith("oidc: "));

        for (AllowEntry entry : store.allowlist()) {
            store.disallow(entry.id);
        }
        assertThatThrownBy(() -> signIn(closed).finish(true, ready(), "the-code", null, now))
                .as("off the list").hasMessage(AccessPolicy.REFUSED);
    }

    @Test
    void aRefusedCodeAndMissingEmail() throws Exception {
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        String url = oidc.start();
        String state = query(url, "state");
        tokenJson = tokenResponse(query(url, "nonce"));

        tokenJson = "{\"error\": \"invalid_grant\"}";
        assertThatThrownBy(() -> signIn(open).finish(true, state, "the-code", null, now)).hasMessageContaining("refused");

        email = null;
        emailVerified = null; // the discovery document has no userinfo endpoint, so the provider cannot fill the gap
        // the failed exchange above spent the state, so a new one is needed
        String url2 = oidc.start();
        tokenJson = tokenResponse(query(url2, "nonce"));
        assertThatThrownBy(() -> signIn(open).finish(true, query(url2, "state"), "the-code", null, now)).hasMessageContaining("verified email");

        assertThatThrownBy(() -> signIn(open).finish(false, ready(), "the-code", null, now)).hasMessageContaining("no accounts");
    }

    @Test
    void theMarkerFollowsTheTokenToTheSignInResult() throws Exception {
        provider("admins", "", "");
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        groupClaimValue = Arrays.asList("players", "admins");

        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(result.idTokenRef).isNotBlank();
        assertThat(oidc.takeTokenRef(result.idTokenRef).admin)
                .as("the marker rides the one-use ref to connectUser").isTrue();
    }

    @Test
    void aSignInOutsideTheAdminGroupCarriesNoMarker() throws Exception {
        provider("admins", "", "");
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        groupClaimValue = Arrays.asList("players");

        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(oidc.takeTokenRef(result.idTokenRef).admin).isFalse();
    }

    @Test
    void theMarkerFollowsTheLinkedAccountPathToo() throws Exception {
        provider("admins", "", "");
        AccessPolicy open = AccessPolicy.create(store, "", "open", "", now);
        accounts.add("octavia", "secret1", "old@example.test");
        store.linkOidc("sub-octo", "octo-cat-with-a-long-name", "old@example.test", "octavia", now);
        groupClaimValue = Arrays.asList("admins");

        OidcSignIn.OidcSignInResult result = signIn(open).finish(true, ready(), "the-code", null, now);

        assertThat(result.userName).isEqualTo("octavia");
        assertThat(oidc.takeTokenRef(result.idTokenRef).admin).isTrue();
    }

    @Test
    void namesFitTheRules() {
        assertThat(OidcAuth.suggestName("a")).isEqualTo("a__");
        assertThat(OidcAuth.suggestName("x-y.z")).isEqualTo("x_y_z");
        assertThat(OidcAuth.suggestName(null)).isEqualTo("___");
        assertThat(OidcAuth.suggestName("octo-cat-with-a-long-name")).isEqualTo("octo_cat_with_");
    }
}
