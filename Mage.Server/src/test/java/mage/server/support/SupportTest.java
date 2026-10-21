package mage.server.support;

import mage.server.admin.AdminStore;
import mage.server.admin.RuntimeSettings;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * The ledger, linking a payment to an account, the perks it unlocks, the month's goal and the supporters wall,
 * GitHub sponsorships, staff's link, revoke and grant, the look players show, and forgetting a deleted account.
 */
public class SupportTest {

    static final long OCT_13 = Instant.parse("2026-10-13T12:00:00Z").toEpochMilli();
    static final long DAY = 86_400_000L;

    @TempDir
    Path dir;

    /** accounts by lowercased name */
    private final Map<String, String> names = new HashMap<>();
    /** confirmed emails, lowercased, to their account */
    private final Map<String, String> confirmed = new HashMap<>();
    /** unconfirmed emails: the server never links on these */
    private final Map<String, String> unconfirmed = new HashMap<>();
    private final Map<Long, String> github = new HashMap<>();
    private final Map<String, List<String[]>> career = new HashMap<>();
    private final List<String> thanked = new ArrayList<>();
    private SupportStore store;
    private Support support;
    private RuntimeSettings settings;

    @BeforeEach
    void open() throws Exception {
        store = new SupportStore("jdbc:sqlite:" + dir.resolve("support.db"));
        settings = RuntimeSettings.create(new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db")));
        RuntimeSettings.use(settings);
        support = new Support(store, new Support.Accounts() {
            @Override
            public String name(String userName) {
                return userName == null ? null : names.get(userName.trim().toLowerCase(Locale.ROOT));
            }

            @Override
            public String confirmedEmailOwner(String email) {
                return confirmed.get(email.toLowerCase(Locale.ROOT));
            }

            @Override
            public String githubAccount(long githubId) {
                return github.get(githubId);
            }

            @Override
            public List<String[]> careerUnlocks(String account) {
                return career.getOrDefault(account.toLowerCase(Locale.ROOT), Collections.emptyList());
            }
        }, (name, amount) -> thanked.add(name + (amount == null ? "" : " " + amount)));
        for (String name : Arrays.asList("Ana", "Bo", "Cy")) {
            names.put(name.toLowerCase(Locale.ROOT), name);
        }
        confirmed.put("ana@example.test", "Ana");
        unconfirmed.put("bo@example.test", "Bo");
    }

    @AfterEach
    void close() throws Exception {
        RuntimeSettings.use(null);
        store.close();
    }

    // ---- Ko-fi payloads shaped like the real ones (ko-fi.com/manage/webhooks)

    static String kofiJson(String type, String transaction, String amount, String email, String message, boolean isPublic,
                           boolean subscription, boolean firstSubscription, String tier) {
        return "{\"verification_token\":\"tok-123\",\"message_id\":\"" + UUID.randomUUID() + "\",\"timestamp\":\"2026-10-13T12:00:00Z\","
                + "\"type\":\"" + type + "\",\"is_public\":" + isPublic + ",\"from_name\":\"Jo Supporter\","
                + "\"message\":" + (message == null ? "null" : "\"" + message + "\"") + ",\"amount\":\"" + amount + "\","
                + "\"url\":\"https://ko-fi.com/Home/CoffeeShop?txid=" + transaction + "\",\"email\":\"" + email + "\",\"currency\":\"USD\","
                + "\"is_subscription_payment\":" + subscription + ",\"is_first_subscription_payment\":" + firstSubscription + ","
                + "\"kofi_transaction_id\":\"" + transaction + "\",\"shop_items\":" + ("Shop Order".equals(type)
                ? "[{\"direct_link_code\":\"1a2b3c\",\"variation_name\":\"Sticker\",\"quantity\":1}]" : "null") + ","
                + "\"tier_name\":" + (tier == null ? "null" : "\"" + tier + "\"") + ",\"shipping\":null}";
    }

    static String form(String json) throws Exception {
        return "data=" + URLEncoder.encode(json, StandardCharsets.UTF_8.name());
    }

    static KofiPayload donation(String transaction, String amount, String email, String message) throws Exception {
        return KofiPayload.parse(form(kofiJson("Donation", transaction, amount, email, message, true, false, false, null)));
    }

    @Test
    void kofiPayloadsOfEveryKindAreRead() throws Exception {
        KofiPayload donation = donation("tx-1", "3.00", "jo@example.test", "Keep it up!");
        assertThat(donation.kind()).isEqualTo("donation");
        assertThat(donation.amountCents).isEqualTo(300);
        assertThat(donation.currency).isEqualTo("USD");
        assertThat(donation.verificationToken).isEqualTo("tok-123");
        assertThat(donation.isPublic).isTrue();

        KofiPayload first = KofiPayload.parse(form(kofiJson("Subscription", "tx-2", "5.00", "jo@example.test", null, false, true, true, "Patron")));
        assertThat(first.kind()).isEqualTo("membership");
        assertThat(first.firstSubscription).isTrue();
        assertThat(first.tierName).isEqualTo("Patron");
        assertThat(first.message).isNull();
        KofiPayload renewal = KofiPayload.parse(form(kofiJson("Subscription", "tx-3", "5.00", "jo@example.test", null, false, true, false, "Patron")));
        assertThat(renewal.kind()).isEqualTo("membership");
        assertThat(renewal.firstSubscription).isFalse();

        KofiPayload shop = KofiPayload.parse(form(kofiJson("Shop Order", "tx-4", "12.50", "jo@example.test", null, false, false, false, null)));
        assertThat(shop.kind()).isEqualTo("shop");
        assertThat(shop.amountCents).isEqualTo(1250);

        assertThatThrownBy(() -> KofiPayload.parse("data=not%20json")).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> KofiPayload.parse("nothing=here")).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> KofiPayload.parse(form("{\"amount\":\"3.00\"}"))).hasMessageContaining("kofi_transaction_id");
    }

    // ---- linking

    @Test
    void aCodeInTheMessageLinks() throws Exception {
        String code = support.codeFor("Cy", OCT_13);
        assertThat(code).matches("PM-[23456789ABCDEFGHJKMNPQRSTUVWXYZ]{4}");
        assertThat(support.codeFor("Cy", OCT_13 + DAY)).as("the code stays").isEqualTo(code);
        Payment linked = support.recordKofi(donation("tx-c1", "5.00", "stranger@example.test", "for the server, " + code.toLowerCase(Locale.ROOT)), OCT_13).payment;
        assertThat(linked.account).isEqualTo("Cy");
        assertThat(linked.linkedBy).isEqualTo("auto");
        // without the dash, in any case
        Payment again = support.recordKofi(donation("tx-c2", "5.00", "other@example.test", "code " + code.replace("-", "")), OCT_13).payment;
        assertThat(again.account).isEqualTo("Cy");
    }

    @Test
    void aConfirmedEmailLinks() throws Exception {
        Payment payment = support.recordKofi(donation("tx-e1", "3.00", "ANA@example.test", "thanks"), OCT_13).payment;
        assertThat(payment.account).isEqualTo("Ana");
    }

    @Test
    void anUnconfirmedEmailDoesNotLink() throws Exception {
        assertThat(unconfirmed).containsKey("bo@example.test");
        Payment payment = support.recordKofi(donation("tx-u1", "3.00", "bo@example.test", "hi"), OCT_13).payment;
        assertThat(payment.account).as("Bo never confirmed the address").isNull();
        assertThat(store.unmatched(10)).extracting(p -> p.providerId).containsExactly("tx-u1");
        assertThat(store.perks("Bo")).isEmpty();
    }

    @Test
    void theCodeBeatsTheEmail() throws Exception {
        String code = support.codeFor("Cy", OCT_13);
        Payment payment = support.recordKofi(donation("tx-b1", "3.00", "ana@example.test", "from Ana for Cy " + code), OCT_13).payment;
        assertThat(payment.account).isEqualTo("Cy");
    }

    @Test
    void renewalsFollowTheFirstLink() throws Exception {
        String code = support.codeFor("Cy", OCT_13);
        support.recordKofi(KofiPayload.parse(form(kofiJson("Subscription", "tx-r1", "5.00", "cy.pays@example.test", code, false, true, true, "Patron"))), OCT_13);
        // next month's renewal has no message and an address no account confirmed
        Payment renewal = support.recordKofi(KofiPayload.parse(form(kofiJson("Subscription", "tx-r2", "5.00", "Cy.Pays@example.test", null, false, true, false, "Patron"))),
                OCT_13 + 31 * DAY).payment;
        assertThat(renewal.account).isEqualTo("Cy");
    }

    @Test
    void nothingToGoOnStaysUnmatched() throws Exception {
        Payment payment = support.recordKofi(donation("tx-n1", "3.00", "nobody@example.test", "PM-ZZZZ is not anyone's"), OCT_13).payment;
        assertThat(payment.account).isNull();
        assertThat(payment.linkedBy).isNull();
    }

    @Test
    void theLedgerKeepsNoEmailOrToken() throws Exception {
        Payment payment = support.recordKofi(donation("tx-p1", "3.00", "ana@example.test", "hi"), OCT_13).payment;
        String raw = store.rawPayload(payment.id);
        assertThat(raw).contains("tx-p1").doesNotContain("ana@example.test").doesNotContain("tok-123");
        Payment stored = store.payment(payment.id);
        assertThat(stored.emailHash).hasSize(64).doesNotContain("ana");
        // the raw payload goes after 90 days
        support.daily(OCT_13 + 91 * DAY);
        assertThat(store.rawPayload(payment.id)).isNull();
    }

    @Test
    void aRepeatedDeliveryChangesNothing() throws Exception {
        Support.Recorded first = support.recordKofi(donation("tx-d1", "3.00", "ana@example.test", "hi"), OCT_13);
        Support.Recorded again = support.recordKofi(donation("tx-d1", "3.00", "ana@example.test", "hi"), OCT_13 + 1000);
        assertThat(first.duplicate).isFalse();
        assertThat(again.duplicate).isTrue();
        assertThat(again.payment.id).isEqualTo(first.payment.id);
        assertThat(support.status(null, OCT_13 + 2000).monthCents).isEqualTo(300);
    }

    // ---- perks

    @Test
    void aLinkedPaymentUnlocksTheThankYouSetOnceWithAThankYou() throws Exception {
        support.recordKofi(donation("tx-k1", "3.00", "ana@example.test", "hi"), OCT_13);
        Support.SupportYou you = support.you("Ana", OCT_13);
        assertThat(you.perks).extracting(p -> p.kind + ":" + p.item)
                .containsExactlyInAnyOrder("sleeve:supporter", "playmat:patron", "frame:supporter-ring", "title:patron", "flair:heart", "pet:party-corgi");
        assertThat(you.supporter).isTrue();
        assertThat(you.since).isEqualTo(OCT_13);
        assertThat(you.notice).isTrue();
        assertThat(support.noticeSeen("Ana")).isTrue();
        assertThat(support.noticeSeen("Ana")).isFalse();

        // status lapses after 30 days, perks stay
        Support.SupportYou later = support.you("Ana", OCT_13 + 31 * DAY);
        assertThat(later.supporter).isFalse();
        assertThat(later.perks).hasSize(SupportPerks.THANK_YOU.size());
        // a second payment unlocks nothing new: no second thank-you
        support.recordKofi(donation("tx-k2", "3.00", "ana@example.test", "again"), OCT_13 + 40 * DAY);
        assertThat(support.you("Ana", OCT_13 + 40 * DAY).notice).isFalse();
    }

    @Test
    void thePerkCatalogIsCosmeticOnly() {
        assertThat(SupportPerks.COSMETIC_KINDS).containsExactlyInAnyOrder("sleeve", "playmat", "avatar", "frame", "title", "flair", "pet");
        for (SupportPerks.Perk perk : SupportPerks.THANK_YOU) {
            assertThat(SupportPerks.COSMETIC_KINDS).as(perk.kind + ":" + perk.item).contains(perk.kind);
        }
    }

    // ---- the month

    @Test
    void theMonthCountsUsdPaymentsNotRevokedOnes() throws Exception {
        support.recordKofi(donation("tx-m1", "10.00", "ana@example.test", null), OCT_13);
        support.recordKofi(donation("tx-m2", "5.00", "x@example.test", null), OCT_13 + 1000);
        support.recordKofi(KofiPayload.parse(form(kofiJson("Donation", "tx-m3", "4.00", "y@example.test", null, true, false, false, null)
                .replace("\"currency\":\"USD\"", "\"currency\":\"EUR\""))), OCT_13 + 2000);
        // last month's doesn't count
        support.recordKofi(donation("tx-m4", "50.00", "z@example.test", null), OCT_13 - 20 * DAY);
        Payment refunded = support.recordKofi(donation("tx-m5", "7.00", "w@example.test", null), OCT_13 + 3000).payment;
        support.revoke(refunded.id, "refunded", false, OCT_13 + 4000);

        Support.SupportStatus status = support.status(null, OCT_13 + 5000);
        assertThat(status.monthLabel).isEqualTo("2026-10");
        assertThat(status.monthCents).isEqualTo(1500);
        assertThat(status.supporters).isEqualTo(3);
        assertThat(status.goalCents).isEqualTo(4000);

        Support.SupportLedger ledger = support.ledger("", false, "", OCT_13 + 5000);
        assertThat(ledger.monthCents).isEqualTo(1500);
        assertThat(ledger.otherCurrencies).containsExactly("EUR 4.00");
        assertThat(ledger.payments).hasSize(4);
        assertThat(support.ledger("2026-09", false, "", OCT_13).payments).extracting(p -> p.providerId).containsExactly("tx-m4");
    }

    @Test
    void theSettingsShapeTheStatus() throws Exception {
        assertThat(support.status(null, OCT_13).enabled).as("no page to send players to").isFalse();
        settings.set(RuntimeSettings.SUPPORT_KOFI_URL, "https://ko-fi.com/playmat", "owner", OCT_13);
        settings.set(RuntimeSettings.SUPPORT_GOAL_CENTS, "6000", "owner", OCT_13);
        settings.set(RuntimeSettings.SUPPORT_COSTS, "Server $13.09\nBackups", "owner", OCT_13);
        Support.SupportStatus status = support.status(null, OCT_13 + Support.STATUS_CACHE_MILLIS);
        assertThat(status.enabled).isTrue();
        assertThat(status.kofiUrl).isEqualTo("https://ko-fi.com/playmat");
        assertThat(status.goalCents).isEqualTo(6000);
        assertThat(status.costs).containsExactly("Server $13.09", "Backups");
        assertThat(Support.costs(RuntimeSettings.DEFAULT_COSTS)).hasSize(5).startsWith("Server (Hetzner CAX21) $13.09");
        assertThat(Support.costs("[\"One\", \"Two\"]")).containsExactly("One", "Two");
        assertThatThrownBy(() -> settings.set(RuntimeSettings.SUPPORT_KOFI_URL, "javascript:alert(1)", "owner", OCT_13))
                .isInstanceOf(IllegalArgumentException.class);
        settings.set(RuntimeSettings.SUPPORT_ENABLED, "off", "owner", OCT_13);
        assertThat(support.status(null, OCT_13 + 3 * Support.STATUS_CACHE_MILLIS).enabled).isFalse();
    }

    @Test
    void theWallShowsPublicNamesAndPlayersWhoChoseToAppear() throws Exception {
        // unlinked and public: by its Ko-fi name
        support.recordKofi(KofiPayload.parse(form(kofiJson("Donation", "tx-w1", "3.00", "q@example.test", null, true, false, false, null)
                .replace("Jo Supporter", "Quinn"))), OCT_13);
        // unlinked and private: not shown
        support.recordKofi(KofiPayload.parse(form(kofiJson("Donation", "tx-w2", "3.00", "r@example.test", null, false, false, false, null)
                .replace("Jo Supporter", "Rae"))), OCT_13 + 1);
        // linked: only once the player chooses to appear, and by their account name
        support.recordKofi(donation("tx-w3", "3.00", "ana@example.test", null), OCT_13 + 2);
        assertThat(support.status(null, OCT_13 + 10).publicSupporters).containsExactly("Quinn");
        support.setWall("Ana", true);
        assertThat(support.status(null, OCT_13 + 20).publicSupporters).containsExactly("Ana", "Quinn");
        // the Discord thank-you followed the same rule (amounts off by default)
        assertThat(thanked).containsExactly("Quinn");
    }

    // ---- staff

    @Test
    void staffLinkRevokeAndGrant() throws Exception {
        Payment unmatched = support.recordKofi(donation("tx-s1", "3.00", "bo@example.test", null), OCT_13).payment;
        assertThat(support.ledger("", true, "", OCT_13).payments).extracting(p -> p.id).containsExactly(unmatched.id);
        Payment linked = support.link(unmatched.id, "bo", OCT_13 + 1);
        assertThat(linked.account).isEqualTo("Bo");
        assertThat(linked.linkedBy).isEqualTo("staff");
        assertThat(store.perks("Bo")).hasSize(SupportPerks.THANK_YOU.size());
        assertThatThrownBy(() -> support.link(unmatched.id, "nobody", OCT_13)).hasMessageContaining("no account");

        // a refund without taking perks back
        support.revoke(linked.id, "refund", false, OCT_13 + 2);
        assertThat(store.perks("Bo")).hasSize(SupportPerks.THANK_YOU.size());
        assertThat(support.status(null, OCT_13 + 100_000).monthCents).isZero();

        // a chargeback that takes them back: only the perks this payment granted
        Payment other = support.recordKofi(donation("tx-s2", "3.00", "ana@example.test", null), OCT_13 + 3).payment;
        support.revoke(other.id, "chargeback", true, OCT_13 + 4);
        assertThat(store.perks("Ana")).isEmpty();
        assertThat(store.perks("Bo")).as("Bo's perks came from another payment").hasSize(SupportPerks.THANK_YOU.size());

        support.grant("cy", OCT_13 + 5);
        assertThat(store.perks("Cy")).hasSize(SupportPerks.THANK_YOU.size());
        assertThat(support.you("Cy", OCT_13 + 5).notice).isTrue();
    }

    @Test
    void revokingKeepsPerksAnotherPaymentStandsFor() throws Exception {
        Payment first = support.recordKofi(donation("tx-v1", "3.00", "ana@example.test", null), OCT_13).payment;
        support.recordKofi(donation("tx-v2", "3.00", "ana@example.test", null), OCT_13 + 1);
        support.revoke(first.id, "chargeback", true, OCT_13 + 2);
        assertThat(store.perks("Ana")).hasSize(SupportPerks.THANK_YOU.size());
    }

    // ---- GitHub Sponsors

    static GithubSponsorsEvent sponsorship(String action, long sponsorId, int cents, boolean oneTime, String extra) {
        return GithubSponsorsEvent.parse(GithubSponsorsTest.event(action, sponsorId, cents, oneTime, extra));
    }

    @Test
    void sponsorshipsCountEachMonthTheyRun() throws Exception {
        github.put(7L, "Cy");
        support.recordSponsorship(sponsorship("created", 7, 500, false, ""), OCT_13);
        assertThat(support.status(null, OCT_13).monthCents).isEqualTo(500);
        assertThat(store.perks("Cy")).hasSize(SupportPerks.THANK_YOU.size());
        assertThat(support.you("Cy", OCT_13).supporter).isTrue();
        // next month the daily job adds November's payment
        long nov = Instant.parse("2026-11-02T00:00:00Z").toEpochMilli();
        support.daily(nov);
        assertThat(support.status(null, nov).monthCents).isEqualTo(500);
        // tier change, then a cancellation at the end of the cycle
        support.recordSponsorship(sponsorship("tier_changed", 7, 1000, false, ""), nov + 1);
        support.recordSponsorship(sponsorship("pending_cancellation", 7, 1000, false, ",\"effective_date\":\"2026-12-01T00:00:00+00:00\""), nov + 2);
        assertThat(support.you("Cy", nov + 3).supporter).isTrue();
        long dec = Instant.parse("2026-12-05T00:00:00Z").toEpochMilli();
        support.daily(dec);
        assertThat(support.status(null, dec).monthCents).as("ended before December").isZero();
        assertThat(store.sponsorship(7).active).isFalse();
        assertThat(store.perks("Cy")).as("perks stay").hasSize(SupportPerks.THANK_YOU.size());
    }

    @Test
    void anUnlinkedSponsorLinksWhenTheySignInWithGithub() throws Exception {
        support.recordSponsorship(sponsorship("created", 9, 300, false, ""), OCT_13);
        assertThat(support.ledger("", true, "github", OCT_13).payments).hasSize(1);
        support.recordSponsorship(sponsorship("created", 10, 2500, true, ""), OCT_13 + 1);
        assertThat(store.unmatchedCount()).isEqualTo(2);

        support.githubLinked(9, "Bo", OCT_13 + 2);
        assertThat(store.unmatchedCount()).isEqualTo(1);
        assertThat(store.sponsorship(9).account).isEqualTo("Bo");
        assertThat(store.perks("Bo")).hasSize(SupportPerks.THANK_YOU.size());
        support.githubLinked(10, "Cy", OCT_13 + 3);
        assertThat(store.unmatchedCount()).isZero();
        assertThat(support.status(null, OCT_13 + 100_000).monthCents).isEqualTo(2800);

        support.recordSponsorship(sponsorship("cancelled", 9, 300, false, ""), OCT_13 + 4);
        assertThat(store.sponsorship(9).active).isFalse();
    }

    // ---- looks

    @Test
    void aLookKeepsOnlyWhatTheAccountOwns() throws Exception {
        career.put("cy", Arrays.asList(new String[]{"sleeve", "copper"}, new String[]{"avatar", "owl"}, new String[]{"title", "veteran"},
                new String[]{"playmat", "sand"}));
        PlayerLook wanted = new PlayerLook();
        wanted.sleeve = "supporter";
        wanted.cloth = "patron";
        wanted.avatar = "owl";
        wanted.frame = "supporter-ring";
        wanted.title = "patron";
        wanted.flair = "heart";
        wanted.pet = "party-corgi";
        PlayerLook kept = support.setLook("Cy", wanted, OCT_13);
        assertThat(kept.sleeve).as("no perks yet").isNull();
        assertThat(kept.cloth).isNull();
        assertThat(kept.frame).isNull();
        assertThat(kept.title).isNull();
        assertThat(kept.flair).isNull();
        assertThat(kept.pet).isNull();
        assertThat(kept.avatar).as("a Career unlock").isEqualTo("owl");

        support.grant("Cy", OCT_13);
        kept = support.setLook("Cy", wanted, OCT_13);
        assertThat(Arrays.asList(kept.sleeve, kept.cloth, kept.avatar, kept.frame, kept.title, kept.flair))
                .containsExactly("supporter", "patron", "owl", "supporter-ring", "patron", "heart");
        assertThat(kept.pet).isEqualTo("party-corgi");
        assertThat(support.looks(Collections.singletonList("cy")).get(0).pet).as("kept in the store").isEqualTo("party-corgi");

        PlayerLook free = new PlayerLook();
        free.sleeve = "#20302B";
        free.cloth = "navy";
        free.title = "veteran";
        kept = support.validLook("Cy", free);
        assertThat(kept.sleeve).isEqualTo("#20302b");
        assertThat(kept.cloth).isEqualTo("navy");
        assertThat(kept.title).isEqualTo("veteran");
        PlayerLook made = new PlayerLook();
        made.sleeve = "#ff00ff";
        made.cloth = "rose";
        made.title = "Admin<script>";
        kept = support.validLook("Cy", made);
        assertThat(Arrays.asList(kept.sleeve, kept.cloth, kept.title)).containsOnlyNulls();
    }

    @Test
    void supportersFromBeforePetsGetTheCorgi() throws Exception {
        String url = "jdbc:sqlite:" + dir.resolve("old.db");
        try (SupportStore fresh = new SupportStore(url)) {
            fresh.addPerk("Old", "sleeve", "supporter", OCT_13, 7);
            fresh.addPerk("Old", "flair", "heart", OCT_13, 7);
        }
        // as a version 1 database left it: no pet column, no pet perk
        try (java.sql.Connection raw = java.sql.DriverManager.getConnection(url); java.sql.Statement sql = raw.createStatement()) {
            sql.execute("ALTER TABLE looks DROP COLUMN pet");
            for (String column : new String[]{"arrow", "spark", "stamp", "lands", "badges"}) {
                sql.execute("ALTER TABLE looks DROP COLUMN " + column);
            }
            sql.execute("UPDATE schema_version SET version = 1");
        }
        try (SupportStore migrated = new SupportStore(url)) {
            assertThat(migrated.schemaVersion()).isEqualTo(3);
            assertThat(migrated.perks("old").stream().map(perk -> perk.kind + ":" + perk.item + ":" + perk.paymentId))
                    .contains("pet:party-corgi:7");
        }
    }

    @Test
    void looksAreAskedForByNameAndFollowRevocation() throws Exception {
        Payment payment = support.recordKofi(donation("tx-l1", "3.00", "ana@example.test", null), OCT_13).payment;
        PlayerLook wanted = new PlayerLook();
        wanted.sleeve = "supporter";
        wanted.flair = "heart";
        support.setLook("Ana", wanted, OCT_13);
        List<PlayerLook> looks = support.looks(Arrays.asList("ana", "Bo", "nobody"));
        assertThat(looks).hasSize(1);
        assertThat(looks.get(0).name).isEqualTo("Ana");
        assertThat(looks.get(0).sleeve).isEqualTo("supporter");
        support.revoke(payment.id, "chargeback", true, OCT_13 + 1);
        assertThat(support.looks(Collections.singletonList("Ana"))).as("nothing left to show").isEmpty();
    }

    // ---- deleting an account

    @Test
    void aDeletedAccountsPaymentsStayAnonymized() throws Exception {
        support.recordKofi(donation("tx-f1", "3.00", "ana@example.test", "from Ana"), OCT_13);
        support.setWall("Ana", true);
        Map<String, Object> export = support.export("Ana");
        assertThat(export).containsKeys("code", "payments", "perks", "look", "supportersWall");
        assertThat(new com.google.gson.Gson().toJson(export)).doesNotContain("rawPayload").doesNotContain("emailHash");

        support.forget("Ana");
        Payment kept = store.paymentByProviderId("tx-f1");
        assertThat(kept.amountCents).isEqualTo(300);
        assertThat(Arrays.asList(kept.account, kept.fromName, kept.message, kept.emailHash, kept.rawPayload)).containsOnlyNulls();
        assertThat(store.perks("Ana")).isEmpty();
        assertThat(store.codeOf("Ana")).isNull();
        assertThat(support.status(null, OCT_13 + 100_000).monthCents).isEqualTo(300);
        assertThat(support.status(null, OCT_13 + 100_000).publicSupporters).isEmpty();
    }

    @Test
    void codesUseTheUnambiguousAlphabet() throws Exception {
        List<String> codes = new ArrayList<>();
        for (int i = 0; i < 50; i++) {
            names.put("p" + i, "P" + i);
            codes.add(support.codeFor("P" + i, OCT_13));
        }
        assertThat(codes.stream().distinct().collect(Collectors.toList())).hasSize(50);
        assertThat(String.join("", codes).replace("PM-", "")).doesNotContain("0").doesNotContain("O").doesNotContain("1")
                .doesNotContain("I").doesNotContain("L");
    }
}
