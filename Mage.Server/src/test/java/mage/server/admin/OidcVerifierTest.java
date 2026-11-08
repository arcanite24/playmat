package mage.server.admin;

import com.nimbusds.jose.JOSEException;
import com.nimbusds.jose.JWSAlgorithm;
import com.nimbusds.jose.JWSHeader;
import com.nimbusds.jose.crypto.ECDSASigner;
import com.nimbusds.jose.crypto.RSASSASigner;
import com.nimbusds.jose.jwk.Curve;
import com.nimbusds.jose.jwk.ECKey;
import com.nimbusds.jose.jwk.JWK;
import com.nimbusds.jose.jwk.JWKSet;
import com.nimbusds.jose.jwk.KeyUse;
import com.nimbusds.jose.jwk.RSAKey;
import com.nimbusds.jwt.JWTClaimsSet;
import com.nimbusds.jwt.SignedJWT;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.interfaces.ECPublicKey;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.ECGenParameterSpec;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.Date;
import java.util.List;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatCode;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Verifying the ID token the identity provider returns: its signature against the keys the provider
 * publishes (re-fetched at most once a minute when one is missing), and its issuer, audience, expiry and
 * nonce. Every failure is a message the player can read.
 */
public class OidcVerifierTest {

    private static final String ISSUER = "https://id.example.com/";
    private static final String JWKS_URI = "https://id.example.com/.well-known/jwks.json";
    private static final String CLIENT_ID = "playmat";

    private long now = 1_000_000;
    private final List<String> requests = new ArrayList<>();
    private String jwksJson;
    private OidcAuth.Discovery discovery;
    private OidcVerifier verifier;

    /** published in the JWKS as "k1" */
    private KeyPair rsa;
    /** published in the JWKS as "k2" */
    private KeyPair ec;
    /** an RSA key the provider did not publish */
    private KeyPair stranger;

    @BeforeEach
    void setUp() throws Exception {
        rsa = keyPair("RSA", null, 2048);
        ec = keyPair("EC", new ECGenParameterSpec("secp256r1"), null);
        stranger = keyPair("RSA", null, 2048);
        jwksJson = publishedJson();
        discovery = new OidcAuth.Discovery(ISSUER, "https://id.example.com/authorize", "https://id.example.com/token",
                JWKS_URI, null, null, null);
        verifier = new OidcVerifier((method, url, token, form) -> {
            requests.add(method + " " + url);
            return jwksJson;
        }, () -> now, CLIENT_ID);
    }

    private static KeyPair keyPair(String algorithm, ECGenParameterSpec curve, Integer rsaBits) throws Exception {
        KeyPairGenerator generator = KeyPairGenerator.getInstance(algorithm);
        if (curve != null) {
            generator.initialize(curve);
        } else {
            generator.initialize(rsaBits);
        }
        return generator.generateKeyPair();
    }

    private JWK publishedRsa() {
        return new RSAKey.Builder((RSAPublicKey) rsa.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.RS256).keyID("k1").build();
    }

    private JWK publishedEc() {
        return new ECKey.Builder(Curve.P_256, (ECPublicKey) ec.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.ES256).keyID("k2").build();
    }

    private String publishedJson() {
        return new JWKSet(Arrays.asList(publishedRsa(), publishedEc())).toString();
    }

    private JWTClaimsSet claims(String sub, String nonce, long lifetimeSeconds) {
        return new JWTClaimsSet.Builder()
                .issuer(ISSUER)
                .audience(CLIENT_ID)
                .subject(sub)
                .claim("nonce", nonce)
                .issueTime(new Date(now))
                .expirationTime(new Date(now + lifetimeSeconds * 1000))
                .build();
    }

    private String token(JWSAlgorithm algorithm, String kid, KeyPair key, JWTClaimsSet claims) throws JOSEException {
        JWSHeader.Builder header = new JWSHeader.Builder(algorithm);
        if (kid != null) {
            header.keyID(kid);
        }
        SignedJWT jwt = new SignedJWT(header.build(), claims);
        String name = algorithm.getName();
        if (name.startsWith("RS") || name.startsWith("PS")) {
            jwt.sign(new RSASSASigner(key.getPrivate()));
        } else if (name.startsWith("ES")) {
            jwt.sign(new ECDSASigner((java.security.interfaces.ECPrivateKey) key.getPrivate(), Curve.P_256));
        }
        return jwt.serialize();
    }

    /** a token that parses but was signed with something the provider would never do */
    private String forged(String algorithm, JWTClaimsSet claims) {
        String header = base64Url("{\"alg\": \"" + algorithm + "\", \"typ\": \"JWT\"}");
        return header + "." + base64Url(claims.toString()) + "." + base64Url("not a real signature");
    }

    private static String base64Url(String text) {
        return Base64.getUrlEncoder().withoutPadding().encodeToString(text.getBytes(StandardCharsets.UTF_8));
    }

    @Test
    void rs256Ps256AndEs256TokensVerifyAgainstThePublishedKeys() throws Exception {
        assertThatCode(() -> verifier.verify(token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", 300)), "n1", discovery))
                .doesNotThrowAnyException();
        assertThatCode(() -> verifier.verify(token(JWSAlgorithm.PS256, "k1", rsa, claims("octocat", "n1", 300)), "n1", discovery))
                .doesNotThrowAnyException();
        assertThatCode(() -> verifier.verify(token(JWSAlgorithm.ES256, "k2", ec, claims("octocat", "n1", 300)), "n1", discovery))
                .doesNotThrowAnyException();

        assertThat(requests).containsExactly("GET " + JWKS_URI);
    }

    @Test
    void theFetchedKeysAreReusedUntilAKeyIsMissing() throws Exception {
        verifier.verify(token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", 300)), "n1", discovery);
        now += 30_000; // inside the refresh throttle, and every key the token names is cached
        verifier.verify(token(JWSAlgorithm.ES256, "k2", ec, claims("octocat", "n2", 300)), "n2", discovery);

        assertThat(requests).as("the key set is cached, not re-fetched per token").hasSize(1);
    }

    @Test
    void aKeyTheProviderHasNotPublishedIsFetchedAgainAfterTheThrottle() throws Exception {
        jwksJson = new JWKSet(publishedEc()).toString(); // the provider rotated away k1
        String rotated = token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", 300));

        assertThatThrownBy(() -> verifier.verify(rotated, "n1", discovery)).hasMessageContaining("isn't available");
        assertThat(requests).hasSize(1);

        now += 30_000;
        assertThatThrownBy(() -> verifier.verify(rotated, "n1", discovery)).hasMessageContaining("isn't available");
        assertThat(requests).as("no re-fetch inside the throttle").hasSize(1);

        jwksJson = publishedJson(); // the rotation propagated
        now += 30_000;
        assertThatCode(() -> verifier.verify(rotated, "n1", discovery)).doesNotThrowAnyException();
        assertThat(requests).hasSize(2);
    }

    @Test
    void unreadableKeysAreRefusedUntilTheThrottleLapses() throws Exception {
        jwksJson = "the provider's keys are unreadable";
        String rs = token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", 300));

        assertThatThrownBy(() -> verifier.verify(rs, "n1", discovery)).hasMessageContaining("couldn't be read");
        now += 30_000;
        assertThatThrownBy(() -> verifier.verify(rs, "n1", discovery)).hasMessageContaining("couldn't be read");
        assertThat(requests).as("a failed fetch is throttled").hasSize(1);

        jwksJson = publishedJson();
        now += 30_000;
        assertThatCode(() -> verifier.verify(rs, "n1", discovery)).doesNotThrowAnyException();
        assertThat(requests).hasSize(2);
    }

    @Test
    void aTokenWithNoKidVerifiesAgainstTheOnlyPublishedKey() throws Exception {
        jwksJson = new JWKSet(publishedRsa()).toString();

        assertThatCode(() -> verifier.verify(token(JWSAlgorithm.RS256, null, rsa, claims("octocat", "n1", 300)), "n1", discovery))
                .doesNotThrowAnyException();
    }

    @Test
    void aSignatureTheProviderKeyDoesNotMakeIsRefused() throws Exception {
        String signedByStranger = token(JWSAlgorithm.RS256, "k1", stranger, claims("octocat", "n1", 300));

        assertThatThrownBy(() -> verifier.verify(signedByStranger, "n1", discovery)).hasMessageContaining("didn't check out");
    }

    @Test
    void theClaimsHaveToMatch() throws Exception {
        String good = token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", 300));
        assertThat(verifier.verify(good, "n1", discovery).getSubject()).isEqualTo("octocat");

        String wrongIssuer = token(JWSAlgorithm.RS256, "k1", rsa,
                new JWTClaimsSet.Builder().issuer("https://elsewhere.example/").audience(CLIENT_ID).subject("octocat").claim("nonce", "n1")
                        .issueTime(new Date(now)).expirationTime(new Date(now + 300_000)).build());
        assertThatThrownBy(() -> verifier.verify(wrongIssuer, "n1", discovery)).hasMessageContaining("doesn't come from the expected");

        String wrongAudience = token(JWSAlgorithm.RS256, "k1", rsa,
                new JWTClaimsSet.Builder().issuer(ISSUER).audience("someone-else").subject("octocat").claim("nonce", "n1")
                        .issueTime(new Date(now)).expirationTime(new Date(now + 300_000)).build());
        assertThatThrownBy(() -> verifier.verify(wrongAudience, "n1", discovery)).hasMessageContaining("wasn't issued for this server");

        String expired = token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", -120));
        assertThatThrownBy(() -> verifier.verify(expired, "n1", discovery)).hasMessageContaining("expired");

        String aBitOld = token(JWSAlgorithm.RS256, "k1", rsa, claims("octocat", "n1", -30)); // older than ideal, inside the skew
        assertThatCode(() -> verifier.verify(aBitOld, "n1", discovery)).doesNotThrowAnyException();

        assertThatThrownBy(() -> verifier.verify(good, "another-nonce", discovery)).hasMessageContaining("doesn't match this sign-in");
    }

    @Test
    void signaturesWeCannotVerifyAreRefusedWithoutFetching() {
        JWTClaimsSet bare = new JWTClaimsSet.Builder().subject("octocat").build();

        // nimbus 9.x cannot even parse a "none" header, so such a token is refused as unreadable
        assertThatThrownBy(() -> verifier.verify(forged("none", bare), "n1", discovery)).hasMessageContaining("couldn't be read");
        assertThatThrownBy(() -> verifier.verify(forged("HS256", bare), "n1", discovery)).hasMessageContaining("isn't one we accept");
        assertThatThrownBy(() -> verifier.verify(forged("EdDSA", bare), "n1", discovery)).hasMessageContaining("unsupported algorithm");

        assertThat(requests).as("no key fetch for a signature we won't even check").isEmpty();
    }

    @Test
    void anAlgorithmTheProviderDoesNotPromiseIsRefused() throws Exception {
        OidcAuth.Discovery promisesRsaOnly = new OidcAuth.Discovery(ISSUER, discovery.authorizationEndpoint, discovery.tokenEndpoint,
                discovery.jwksUri, null, null, Arrays.asList("RS256"));
        String es = token(JWSAlgorithm.ES256, "k2", ec, claims("octocat", "n1", 300));

        assertThatThrownBy(() -> verifier.verify(es, "n1", promisesRsaOnly)).hasMessageContaining("doesn't promise");
    }

    @Test
    void somethingThatIsNotATokenIsRefused() {
        assertThatThrownBy(() -> verifier.verify("definitely not a token", "n1", discovery)).hasMessageContaining("couldn't be read");
        assertThat(requests).isEmpty();
    }
}
