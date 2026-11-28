package mage.server.websocket.service.deckimport;

import mage.cards.decks.DeckCardInfo;
import org.junit.Assert;
import org.junit.Test;

import java.io.IOException;
import java.io.InputStream;
import java.io.ByteArrayOutputStream;
import java.net.URI;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Deck sources against responses recorded from the real sites (2026-10-07), in src/test/resources/deckimport.
 * No network: a fake fetcher serves the recordings.
 */
public class DeckImportSourcesTest {

    static String fixture(String path) {
        return fixture(path, StandardCharsets.UTF_8);
    }

    static String fixture(String path, Charset charset) {
        try (InputStream in = DeckImportSourcesTest.class.getResourceAsStream("/deckimport/" + path)) {
            Assert.assertNotNull("missing fixture " + path, in);
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buffer = new byte[8192];
            int read;
            while ((read = in.read(buffer)) != -1) {
                out.write(buffer, 0, read);
            }
            return new String(out.toByteArray(), charset);
        } catch (IOException e) {
            throw new AssertionError(e);
        }
    }

    private static int count(List<DeckCardInfo> cards) {
        return cards.stream().mapToInt(DeckCardInfo::getAmount).sum();
    }

    private static List<String> names(List<DeckCardInfo> cards) {
        return cards.stream().map(DeckCardInfo::getCardName).collect(Collectors.toList());
    }

    private static DeckImportException.Reason reasonOf(ThrowingRunnable action) {
        try {
            action.run();
        } catch (DeckImportException e) {
            return e.getReason();
        }
        Assert.fail("expected a DeckImportException");
        return null;
    }

    interface ThrowingRunnable {
        void run() throws DeckImportException;
    }

    // Archidekt

    @Test
    public void archidektCommanderDeck() throws Exception {
        ImportedDeck deck = ArchidektSource.read(fixture("archidekt/commander-7031486.json"), "7031486");
        Assert.assertEquals("Buffs by Hans", deck.getName());
        Assert.assertEquals("SalubriousSnail", deck.getAuthor());
        Assert.assertEquals("Variant Magic - Commander", deck.getFormat());
        Assert.assertEquals(1, count(deck.getCommanders()));
        Assert.assertEquals(99, count(deck.getMain()));
        Assert.assertTrue(deck.getSide().isEmpty());
        Assert.assertNotNull(deck.getRemoteUpdatedAt());
        Assert.assertNotNull("the featured card is the cover", deck.getCover());
        DeckCardInfo first = deck.getMain().get(0);
        Assert.assertEquals("BRO", first.getSetCode());
        Assert.assertEquals("182", first.getCardNumber());
    }

    @Test
    public void archidektSideboardAndMaybeboard() throws Exception {
        ImportedDeck deck = ArchidektSource.read(fixture("archidekt/modern-sideboard-maybeboard-3872725.json"), "3872725");
        Assert.assertEquals("Constructed - Modern", deck.getFormat());
        Assert.assertEquals(58, count(deck.getMain()));
        Assert.assertEquals(17, count(deck.getSide()));
        Assert.assertTrue("double-faced and adventure cards keep both names", names(deck.getMain()).contains("Brazen Borrower // Petty Theft"));
    }

    @Test
    public void archidektCompanion() throws Exception {
        ImportedDeck deck = ArchidektSource.read(fixture("archidekt/companion-13235048.json"), "13235048");
        Assert.assertEquals(names(deck.getCompanion()), java.util.Collections.singletonList("Lurrus of the Dream-Den"));
        Assert.assertFalse(names(deck.getSide()).contains("Lurrus of the Dream-Den"));
        Assert.assertNull("custom formats have no XMage deck type", deck.getFormat());
    }

    @Test
    public void archidektMissingDeck() {
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, reasonOf(() -> ArchidektSource.read(fixture("archidekt/not-found.json"), "999999999")));
        Assert.assertEquals(DeckImportException.Reason.SITE_CHANGED, reasonOf(() -> ArchidektSource.read("<html>maintenance</html>", "1")));
    }

    @Test
    public void archidektLinks() {
        ArchidektSource source = new ArchidektSource();
        Assert.assertEquals("7031486", source.deckId(URI.create("https://archidekt.com/decks/7031486/buffs_by_hans")));
        Assert.assertEquals("7031486", source.deckId(URI.create("https://archidekt.com/api/decks/7031486/")));
        Assert.assertNull(source.deckId(URI.create("https://archidekt.com/search/decks")));
    }

    // MTGTop8

    @Test
    public void mtgtop8Deck() throws Exception {
        ImportedDeck deck = MtgTop8Source.read(fixture("mtgtop8/896184.dec", StandardCharsets.ISO_8859_1),
                fixture("mtgtop8/event-91675-896184.html", StandardCharsets.ISO_8859_1), "896184", "91675", "MO");
        Assert.assertEquals("5c Domain Aggro", deck.getName());
        Assert.assertEquals("Ryuumei", deck.getAuthor());
        Assert.assertEquals("Constructed - Modern", deck.getFormat());
        Assert.assertEquals(60, count(deck.getMain()));
        Assert.assertEquals(15, count(deck.getSide()));
        Assert.assertTrue(names(deck.getSide()).contains("Wear // Tear"));
        DeckCardInfo bolt = deck.getMain().stream().filter(card -> card.getCardName().equals("Lightning Bolt")).findFirst().orElse(null);
        Assert.assertNotNull(bolt);
        Assert.assertEquals("M11", bolt.getSetCode());
        Assert.assertNull(bolt.getCardNumber());
        Assert.assertEquals("https://mtgtop8.com/event?e=91675&d=896184&f=MO", deck.getUrl());
    }

    @Test
    public void mtgtop8FormatFromPageWhenTheLinkHasNone() throws Exception {
        ImportedDeck deck = MtgTop8Source.read(fixture("mtgtop8/896184.dec", StandardCharsets.ISO_8859_1),
                fixture("mtgtop8/event-91675-896184.html", StandardCharsets.ISO_8859_1), "896184", "91675", null);
        Assert.assertEquals("Constructed - Modern", deck.getFormat());
    }

    @Test
    public void mtgtop8WithoutThePage() throws Exception {
        ImportedDeck deck = MtgTop8Source.read(fixture("mtgtop8/896184.dec", StandardCharsets.ISO_8859_1), null, "896184", null, null);
        Assert.assertEquals("MTGTop8 deck 896184", deck.getName());
        Assert.assertEquals(75, deck.cardCount());
    }

    @Test
    public void mtgtop8MissingDeck() {
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, reasonOf(() -> MtgTop8Source.read("// Deck file created with mtgtop8.com\n", null, "1", null, null)));
        Assert.assertEquals(DeckImportException.Reason.SITE_CHANGED, reasonOf(() -> MtgTop8Source.read("<!DOCTYPE html><html></html>", null, "1", null, null)));
    }

    @Test
    public void mtgtop8Links() {
        MtgTop8Source source = new MtgTop8Source();
        Assert.assertEquals("896184", source.deckId(URI.create("https://mtgtop8.com/event?e=91675&d=896184&f=MO")));
        Assert.assertEquals("896184", source.deckId(URI.create("https://mtgtop8.com/dec?d=896184")));
        Assert.assertNull(source.deckId(URI.create("https://mtgtop8.com/event?e=91675")));
        Assert.assertNull(source.deckId(URI.create("https://mtgtop8.com/format?f=MO")));
    }

    // Scryfall

    @Test
    public void scryfallCommanderDeck() throws Exception {
        ImportedDeck deck = ScryfallSource.read(fixture("scryfall/commander-e7aceb4c.json"), "e7aceb4c-29d5-49f5-9a49-c24f64da264b");
        Assert.assertEquals("[mtg-parser] 3 Amigos", deck.getName());
        Assert.assertEquals("gorila", deck.getAuthor());
        Assert.assertEquals("Variant Magic - Commander", deck.getFormat());
        Assert.assertEquals(2, count(deck.getCommanders()));
        Assert.assertEquals(java.util.Collections.singletonList("Obosh, the Preypiercer"), names(deck.getCompanion()));
        Assert.assertTrue(deck.getSide().isEmpty());
        Assert.assertEquals("an empty maybeboard row is skipped", 0, deck.getMaybeboardCount());
        Assert.assertNotNull(deck.getCover());
        Assert.assertEquals("CMR", deck.getCover().getSetCode());
    }

    @Test
    public void scryfallMissingDeck() {
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, reasonOf(() -> ScryfallSource.read(fixture("scryfall/not-found.json"), "0")));
    }

    @Test
    public void scryfallLinks() {
        ScryfallSource source = new ScryfallSource();
        Assert.assertEquals("e7aceb4c-29d5-49f5-9a49-c24f64da264b",
                source.deckId(URI.create("https://scryfall.com/@gorila/decks/e7aceb4c-29d5-49f5-9a49-c24f64da264b")));
        Assert.assertEquals("e7aceb4c-29d5-49f5-9a49-c24f64da264b",
                source.deckId(URI.create("https://api.scryfall.com/decks/E7ACEB4C-29D5-49F5-9A49-C24F64DA264B/export/json")));
        Assert.assertNull(source.deckId(URI.create("https://scryfall.com/card/m11/149/lightning-bolt")));
    }

    // TCGplayer

    @Test
    public void tcgplayerCommanderDeck() throws Exception {
        ImportedDeck deck = TcgplayerSource.read(fixture("tcgplayer/commander-496887.json"), "496887");
        Assert.assertEquals("Malcolm and Vial Smasher", deck.getName());
        Assert.assertEquals("Everett Delfel", deck.getAuthor());
        Assert.assertEquals("Variant Magic - Commander", deck.getFormat());
        Assert.assertEquals(2, count(deck.getCommanders()));
        Assert.assertEquals(98, count(deck.getMain()));
        Assert.assertEquals("5th-8th at SCG Con Las Vegas cEDH $8K", deck.getDescription());
        Assert.assertTrue("TCGplayer has no collector numbers", deck.getMain().stream().allMatch(card -> card.getCardNumber() == null));
        Assert.assertEquals("https://infinite.tcgplayer.com/magic-the-gathering/deck/Malcolm-and-Vial-Smasher/496887", deck.getUrl());
    }

    @Test
    public void tcgplayerSideboard() throws Exception {
        ImportedDeck deck = TcgplayerSource.read(fixture("tcgplayer/modern-496500.json"), "496500");
        Assert.assertEquals("Constructed - Modern", deck.getFormat());
        Assert.assertEquals(60, count(deck.getMain()));
        Assert.assertEquals(15, count(deck.getSide()));
    }

    @Test
    public void tcgplayerMissingDeck() {
        Assert.assertEquals(DeckImportException.Reason.NOT_FOUND, reasonOf(() -> TcgplayerSource.read(fixture("tcgplayer/not-found.json"), "560000")));
    }

    // ManaBox

    @Test
    public void manaboxCommanderDeck() throws Exception {
        ImportedDeck deck = ManaboxSource.read(fixture("manabox/commander-AZ6V4n88fhahOvNRFHbXTQ.json"), "AZ6V4n88fhahOvNRFHbXTQ");
        Assert.assertEquals("Flash Fire", deck.getName());
        Assert.assertEquals("Variant Magic - Commander", deck.getFormat());
        Assert.assertEquals(names(deck.getCommanders()), java.util.Collections.singletonList("Surrak Dragonclaw"));
        Assert.assertEquals(99, count(deck.getMain()));
        Assert.assertEquals(3, count(deck.getSide()));
        Assert.assertEquals(Long.valueOf(1786314484436L), deck.getRemoteUpdatedAt());
        Assert.assertNotNull(deck.getCover());
    }

    @Test
    public void manaboxConstructedDeck() throws Exception {
        ImportedDeck deck = ManaboxSource.read(fixture("manabox/constructed-NzWns2ueTeO8p0S0mDEkVw.json"), "NzWns2ueTeO8p0S0mDEkVw");
        Assert.assertNull(deck.getFormat());
        Assert.assertEquals(66, count(deck.getMain()));
        Assert.assertTrue(deck.getCommanders().isEmpty());
    }

    @Test
    public void manaboxChangedShape() {
        Assert.assertEquals(DeckImportException.Reason.SITE_CHANGED, reasonOf(() -> ManaboxSource.read("{\"id\":\"x\"}", "x")));
    }
}
