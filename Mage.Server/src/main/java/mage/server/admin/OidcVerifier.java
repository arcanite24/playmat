package mage.server.admin;

import com.nimbusds.jose.JOSEException;
import com.nimbusds.jose.JWSAlgorithm;
import com.nimbusds.jose.JWSHeader;
import com.nimbusds.jose.JWSObject;
import com.nimbusds.jose.JWSVerifier;
import com.nimbusds.jose.crypto.ECDSAVerifier;
import com.nimbusds.jose.crypto.RSASSAVerifier;
import com.nimbusds.jose.jca.JCAContext;
import com.nimbusds.jose.jwk.ECKey;
import com.nimbusds.jose.jwk.JWK;
import com.nimbusds.jose.jwk.JWKSet;
import com.nimbusds.jose.jwk.RSAKey;
import com.nimbusds.jose.proc.JWSKeySelector;
import com.nimbusds.jose.proc.JWSVerifierFactory;
import com.nimbusds.jose.proc.SecurityContext;
import com.nimbusds.jwt.JWTClaimsSet;
import com.nimbusds.jwt.SignedJWT;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.security.Key;
import java.security.interfaces.ECPublicKey;
import java.security.interfaces.RSAPublicKey;
import java.text.ParseException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * Verifies the ID token the provider returns for a sign-in: its signature against the keys the provider
 * published (its JWKS, fetched through the same {@link OidcAuth.Http} seam the provider client uses — never
 * the library's own HTTP), and its issuer, audience, expiry and nonce. Every failure is an
 * IllegalArgumentException with a message the player can read.
 */
final class OidcVerifier {

    private static final Logger logger = Logger.getLogger(OidcVerifier.class);

    /**
     * What a provider may promise for ID tokens: the asymmetric families only. "none" and HS* are algorithm
     * confusion; EdDSA would need Bouncy Castle on the Java 8 baseline, so it is refused with a setup note.
     */
    static final Set<String> ASYMMETRIC = Collections.unmodifiableSet(new HashSet<>(Arrays.asList(
            "RS256", "RS384", "RS512", "PS256", "PS384", "PS512", "ES256", "ES384", "ES512")));

    /** an ID token may be up to this old (providers sign them short; Pocket ID: an hour) */
    static final long SKEW_MILLIS = TimeUnit.MINUTES.toMillis(1);
    /** a JWKS re-fetch after a missing key, at most this often (a flood of bad tokens must not hammer the provider) */
    static final long REFRESH_MILLIS = TimeUnit.MINUTES.toMillis(1);
    /** how long the provider's published keys are trusted before they are fetched again, kid miss or not */
    static final long MAX_KEYS_AGE_MILLIS = 24L * 60 * 60 * 1000;

    private final OidcAuth.Http http;
    private final LongSupplier clock;
    private final String clientId;
    private final JWSVerifierFactory verifiers = new Verifiers();

    /** the provider's public keys, as last fetched */
    private volatile JWKSet keys;
    private volatile long fetchedAt;

    OidcVerifier(OidcAuth.Http http, LongSupplier clock, String clientId) {
        this.http = http;
        this.clock = clock;
        this.clientId = clientId;
    }

    /**
     * Checks the ID token's signature and its claims.
     *
     * @return the verified claims
     */
    JWTClaimsSet verify(String idToken, String nonce, OidcAuth.Discovery discovery) {
        JWSObject jws;
        try {
            jws = SignedJWT.parse(idToken);
        } catch (ParseException | RuntimeException e) {
            throw new IllegalArgumentException("The sign-in token couldn't be read. Start again.");
        }
        JWSHeader header = jws.getHeader();
        String alg = header.getAlgorithm().getName();
        if ("none".equalsIgnoreCase(alg) || alg.startsWith("HS")) {
            throw new IllegalArgumentException("The sign-in token's signature isn't one we accept. Start again.");
        }
        if (alg.startsWith("Ed")) {
            throw new IllegalArgumentException("EdDSA is an unsupported algorithm here: ask your identity provider to "
                    + "sign ID tokens with RS256 or ES256 instead (Pocket ID: `pocket-id key-rotate`).");
        }
        if (!promised(alg, discovery)) {
            throw new IllegalArgumentException("Your identity provider signed the token with an algorithm it doesn't promise. Start again.");
        }
        boolean verified = false;
        for (Key key : selectKeys(header, discovery)) {
            try {
                JWSVerifier verifier = verifiers.createJWSVerifier(header, key);
                if (verifier != null && jws.verify(verifier)) {
                    verified = true;
                    break;
                }
            } catch (JOSEException | RuntimeException e) {
                logger.info("The token didn't verify with a provider key: " + e.getMessage());
            }
        }
        if (!verified) {
            throw new IllegalArgumentException("The sign-in token's signature didn't check out. Start again.");
        }
        JWTClaimsSet claims;
        try {
            claims = ((SignedJWT) jws).getJWTClaimsSet();
        } catch (ParseException | RuntimeException e) {
            throw new IllegalArgumentException("The sign-in token's claims couldn't be read. Start again.");
        }
        long now = clock.getAsLong();
        if (!discovery.issuer.equals(claims.getIssuer())) {
            throw new IllegalArgumentException("The sign-in token doesn't come from the expected identity provider. Start again.");
        }
        List<String> audience = claims.getAudience();
        if (audience == null || !audience.contains(clientId)) {
            throw new IllegalArgumentException("The sign-in token wasn't issued for this server. Start again.");
        }
        if (claims.getExpirationTime() == null || claims.getExpirationTime().getTime() <= now - SKEW_MILLIS) {
            throw new IllegalArgumentException("The sign-in token has expired. Start again.");
        }
        if (!nonce.equals(claims.getClaim("nonce"))) {
            throw new IllegalArgumentException("The sign-in token doesn't match this sign-in. Start again.");
        }
        return claims;
    }

    /**
     * Whether the token's algorithm is one both the provider promises (its discovery document) and we can
     * verify: the intersection of the two, or just ours when the document doesn't list any.
     */
    private boolean promised(String alg, OidcAuth.Discovery discovery) {
        if (!ASYMMETRIC.contains(alg)) {
            return false;
        }
        List<String> supported = discovery.algs;
        return supported == null || supported.isEmpty() || supported.contains(alg);
    }

    /**
     * The provider's public keys a token may be verified with, from the discovery document's JWKS: the key
     * the token's kid names, or every key the provider published when it names none. A missing key is
     * fetched again from the provider — it may have rotated its keys, or the last fetch failed — at most
     * once a minute, so a flood of bad tokens can't hammer the provider.
     */
    private List<? extends Key> selectKeys(JWSHeader header, OidcAuth.Discovery discovery) {
        long now = clock.getAsLong();
        JWKSet set = keys;
        String kid = header.getKeyID();
        boolean noKid = kid == null || kid.trim().isEmpty();
        if (set != null && now - fetchedAt >= MAX_KEYS_AGE_MILLIS) {
            // a key the provider retired must stop verifying, and a kid-less token's key may have rotated
            try {
                set = fetchKeys(discovery);
                keys = set;
                fetchedAt = now;
            } catch (IllegalArgumentException e) {
                logger.warn("The provider's keys couldn't be refreshed: " + e.getMessage());
                fetchedAt = now - MAX_KEYS_AGE_MILLIS + REFRESH_MILLIS; // keep the old set, try again in a minute
            }
        }
        if (set == null) {
            if (fetchedAt == 0 || now - fetchedAt >= REFRESH_MILLIS) {
                fetchedAt = now;
                try {
                    JWKSet fresh = fetchKeys(discovery);
                    keys = fresh;
                    set = fresh;
                } catch (IllegalArgumentException e) {
                    logger.warn("The provider's keys couldn't be fetched: " + e.getMessage());
                }
            }
            if (set == null) {
                throw new IllegalArgumentException("The identity provider's signing keys couldn't be read. Try again in a minute.");
            }
        } else if (!noKid && set.getKeyByKeyId(kid) == null && now - fetchedAt >= REFRESH_MILLIS) {
            fetchedAt = now;
            try {
                JWKSet fresh = fetchKeys(discovery);
                keys = fresh;
                set = fresh;
            } catch (IllegalArgumentException e) {
                logger.warn("The provider's keys couldn't be re-fetched: " + e.getMessage());
            }
        }
        JWK wanted = noKid ? null : set.getKeyByKeyId(kid);
        return new KeySelector(set, wanted).selectJWSKeys(header, null);
    }

    /**
     * A token's key from the provider's published set: the one its kid names, or the set's only key when it
     * names none. (The 9.x shape of the plan's JWKKeySelector: a {@link JWSKeySelector} over the cached set.)
     */
    private static final class KeySelector implements JWSKeySelector<SecurityContext> {
        private final JWKSet set;
        private final JWK wanted;

        KeySelector(JWKSet set, JWK wanted) {
            this.set = set;
            this.wanted = wanted;
        }

        @Override
        public List<? extends Key> selectJWSKeys(JWSHeader header, SecurityContext context) {
            List<Key> chosen = new ArrayList<>();
            if (wanted != null) {
                addPublic(chosen, wanted);
            } else {
                String kid = header.getKeyID();
                if (kid == null || kid.trim().isEmpty()) {
                    for (JWK key : set.getKeys()) {
                        addPublic(chosen, key);
                    }
                }
            }
            if (chosen.isEmpty()) {
                throw new IllegalArgumentException("The identity provider's key for this token isn't available. Try again in a minute.");
            }
            return chosen;
        }

        private static void addPublic(List<Key> chosen, JWK key) {
            try {
                if (key instanceof RSAKey) {
                    chosen.add(((RSAKey) key).toRSAPublicKey());
                } else if (key instanceof ECKey) {
                    chosen.add(((ECKey) key).toECPublicKey());
                }
            } catch (JOSEException | RuntimeException e) {
                logger.warn("A provider key couldn't be read: " + e.getMessage());
            }
        }
    }

    /**
     * The verifier for a token and one of the provider's public keys, by the token's algorithm family
     * (the 9.x shape of the plan's JWSTrustStore: a {@link JWSVerifierFactory} over the chosen key).
     */
    private static final class Verifiers implements JWSVerifierFactory {

        @Override
        public JCAContext getJCAContext() {
            return new JCAContext();
        }

        @Override
        public Set<JWSAlgorithm> supportedJWSAlgorithms() {
            return new HashSet<>(Arrays.asList(
                    JWSAlgorithm.RS256, JWSAlgorithm.RS384, JWSAlgorithm.RS512,
                    JWSAlgorithm.PS256, JWSAlgorithm.PS384, JWSAlgorithm.PS512,
                    JWSAlgorithm.ES256, JWSAlgorithm.ES384, JWSAlgorithm.ES512));
        }

        @Override
        public JWSVerifier createJWSVerifier(JWSHeader header, Key key) throws JOSEException {
            String alg = header.getAlgorithm().getName();
            if (alg.startsWith("RS") || alg.startsWith("PS")) {
                // RSASSAVerifier covers both plain RSA and RSASSA-PSS signatures
                return key instanceof RSAPublicKey ? new RSASSAVerifier((RSAPublicKey) key) : null;
            }
            if (alg.startsWith("ES")) {
                return key instanceof ECPublicKey ? new ECDSAVerifier((ECPublicKey) key) : null;
            }
            return null;
        }
    }

    /** Fetches the provider's published keys (its discovery document's jwks_uri). */
    private JWKSet fetchKeys(OidcAuth.Discovery discovery) {
        String json;
        try {
            json = http.request("GET", discovery.jwksUri, null, null);
        } catch (IOException e) {
            throw new IllegalArgumentException("The identity provider's signing keys couldn't be read. Try again in a minute.");
        }
        try {
            return JWKSet.parse(json);
        } catch (ParseException | RuntimeException e) {
            throw new IllegalArgumentException("The identity provider's signing keys couldn't be read. Try again in a minute.");
        }
    }
}
