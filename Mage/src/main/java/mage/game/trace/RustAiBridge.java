package mage.game.trace;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.constants.ManaType;
import mage.game.Game;
import org.apache.log4j.Logger;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicLong;

/**
 * The Rust AI playing one seat of a game Java hosts (docs/rust/Ai.md): a {@code playmat-ai-bridge} process keeps a
 * Rust copy of the game in step and answers that seat's questions.
 * <p>
 * The game is traced and its answers logged ({@link mage.game.GameOptions#trace}, {@link mage.game.GameOptions#decisions}). At the first
 * trace entry the bridge starts the copy from {@link ShadowStart}; from then on it forwards the other seat's answers
 * and every trace entry, and the seat's dialogs ({@code ComputerPlayerControllableProxy}) ask it for their answers.
 * When the copy differs from Java, the process dies or an answer takes too long, the bridge stops for good and the
 * seat goes back to the Java AI.
 */
public final class RustAiBridge {

    private static final Logger logger = Logger.getLogger(RustAiBridge.class);
    private static final long ANSWER_TIMEOUT_MS = 20_000;
    private static final Map<String, Boolean> SUPPORTED = new ConcurrentHashMap<>();
    private static final Gson GSON = new Gson();
    /** PLAYMAT_AI_DEBUG: the bridge logs each question, and gets Java's fingerprint texts to show a difference */
    private static final boolean DEBUG = System.getenv("PLAYMAT_AI_DEBUG") != null;

    public static final AtomicLong STARTED = new AtomicLong();
    public static final AtomicLong ANSWERED = new AtomicLong();
    public static final AtomicLong STOPPED = new AtomicLong();
    public static final AtomicLong FINISHED = new AtomicLong();

    private final UUID seatId;
    private final DecisionLog log;
    private final String gameName;
    private Process process;
    private Writer out;
    private final BlockingQueue<String> lines = new LinkedBlockingQueue<>();
    private int seat = -1;
    private int sent;
    private volatile boolean active;
    private volatile String failure;
    /** the last question Java put to the seat (for the bridge's messages) */
    private volatile String question = "";
    /** the choice it was, for a CHOOSE_CHOICE */
    private volatile mage.choices.Choice choice;

    /**
     * @param seatId the player the Rust AI plays
     * @param log    the game's decision log (recording)
     */
    public RustAiBridge(UUID seatId, DecisionLog log, String gameName) {
        this.seatId = seatId;
        this.log = log;
        this.gameName = gameName;
    }

    /** the process: {@code -Dxmage.rustAi.bin} or {@code XMAGE_RUST_AI_BIN}, else {@code playmat-ai-bridge} on the path */
    public static String binary() {
        String bin = System.getProperty("xmage.rustAi.bin", System.getenv("XMAGE_RUST_AI_BIN"));
        return bin == null || bin.isEmpty() ? "playmat-ai-bridge" : bin;
    }

    /** Whether the Rust engine has every one of these cards (asked once per name, then remembered). */
    public static boolean supports(Collection<String> names) {
        return missing(names).isEmpty();
    }

    /**
     * The cards of these the Rust engine doesn't have (asked once per name, then remembered); all of them when the
     * bridge can't be asked.
     */
    public static Set<String> missing(Collection<String> names) {
        List<String> unknown = new ArrayList<>();
        for (String n : new LinkedHashSet<>(names)) {
            if (!SUPPORTED.containsKey(n)) {
                unknown.add(n);
            }
        }
        if (!unknown.isEmpty()) {
            try {
                Process p = new ProcessBuilder(binary(), "--check").redirectErrorStream(false).start();
                try (Writer w = new OutputStreamWriter(p.getOutputStream(), StandardCharsets.UTF_8)) {
                    for (String n : unknown) {
                        w.write(n.replace('\n', ' ') + "\n");
                    }
                }
                Set<String> missing = new HashSet<>();
                try (BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream(), StandardCharsets.UTF_8))) {
                    String line;
                    while ((line = r.readLine()) != null) {
                        missing.add(line.trim());
                    }
                }
                if (!p.waitFor(30, TimeUnit.SECONDS) || p.exitValue() != 0) {
                    p.destroyForcibly();
                    return new LinkedHashSet<>(names);
                }
                for (String n : unknown) {
                    SUPPORTED.put(n, !missing.contains(n));
                }
            } catch (IOException e) {
                logger.warn("Rust AI: can't run " + binary() + ": " + e.getMessage());
                return new LinkedHashSet<>(names);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return new LinkedHashSet<>(names);
            }
        }
        Set<String> missing = new TreeSet<>();
        for (String n : names) {
            if (!SUPPORTED.getOrDefault(n, false)) {
                missing.add(n);
            }
        }
        return missing;
    }

    /** the trace's sink: starts the copy at the first entry, forwards the rest, ends with the game */
    public void onTrace(Game game, GameTrace.Entry entry) {
        try {
            if (entry.index == 0) {
                start(game);
            }
            if (active) {
                sendDecisions();
                JsonObject m = new JsonObject();
                m.addProperty("type", "trace");
                m.addProperty("fp", entry.hash);
                if (DEBUG && entry.text != null) {
                    m.addProperty("text", entry.text);
                }
                send(m.toString());
            }
        } catch (RuntimeException e) {
            stop("trace: " + e);
        }
    }

    public boolean isActive() {
        return active;
    }

    public String getFailure() {
        return failure;
    }

    private void start(Game game) {
        seat = StateFingerprint.seats(game).indexOf(seatId);
        if (seat < 0) {
            stop("the seat isn't in the game");
            return;
        }
        for (UUID id : StateFingerprint.seats(game)) {
            if (game.getPlayer(id).getLife() != 20) {
                stop("starting life isn't 20");
                return;
            }
        }
        Map<String, Object> start = ShadowStart.capture(game, log, StateFingerprint.text(game));
        sent = (Integer) start.get("decision");
        Map<String, Object> message = new LinkedHashMap<>();
        message.put("type", "start");
        message.put("seat", seat);
        message.put("start", start);
        // Java's AI players auto-choose targets at level 2 (TargetImpl.tryToAutoChoose)
        message.put("autoTargetAll", true);
        // the other seat's skip settings and pass-until actions aren't the copy's: it logs every pass, and the copy
        // asks it at every priority
        log.recordAutoPasses();
        message.put("everyPriority", true);
        for (UUID id : StateFingerprint.seats(game)) {
            if (!id.equals(seatId) && game.getPlayer(id).getUserData() != null) {
                message.put("poolCheck", game.getPlayer(id).getUserData().confirmEmptyManaPool());
            }
        }
        try {
            process = new ProcessBuilder(binary()).start();
        } catch (IOException e) {
            stop("can't run " + binary() + ": " + e.getMessage());
            return;
        }
        out = new BufferedWriter(new OutputStreamWriter(process.getOutputStream(), StandardCharsets.UTF_8));
        Thread reader = new Thread(() -> {
            try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = r.readLine()) != null) {
                    lines.add(line);
                }
            } catch (IOException ignore) {
            }
            lines.add("{\"type\":\"closed\"}");
        }, "rust-ai " + gameName);
        reader.setDaemon(true);
        reader.start();
        Thread errors = new Thread(() -> {
            try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getErrorStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = r.readLine()) != null) {
                    logger.info("Rust AI " + gameName + ": " + line);
                }
            } catch (IOException ignore) {
            }
        }, "rust-ai-err " + gameName);
        errors.setDaemon(true);
        errors.start();
        game.addPlayerQueryEventListener(event -> {
            if (seatId.equals(event.getPlayerId())) {
                choice = event.getChoice();
                question = event.getQueryType() + " " + (event.getMessage() == null ? "" : event.getMessage())
                        + (DEBUG && event.getChoice() != null ? " " + event.getChoice().getKeyChoices() + event.getChoice().getChoices() : "");
            }
        });
        active = true;
        send(GSON.toJson(message));
        JsonObject ready = next();
        if (ready == null || !"ready".equals(type(ready))) {
            stop("didn't start: " + (ready == null ? failure : ready));
            return;
        }
        STARTED.incrementAndGet();
        logger.info("Rust AI plays seat " + seat + " of " + gameName);
    }

    /**
     * The Rust AI's answer to the question its seat is being asked; null when the bridge has stopped (the dialog
     * then ends, and the seat goes back to the Java AI).
     */
    public synchronized DecisionLog.Answer ask(Game game) {
        if (!active) {
            return null;
        }
        sendDecisions();
        JsonObject ask = new JsonObject();
        ask.addProperty("type", "ask");
        ask.addProperty("question", question);
        send(ask.toString());
        JsonObject a = next();
        if (a == null) {
            return null;
        }
        if (!"answer".equals(type(a))) {
            stop(type(a) + ": " + (a.has("message") ? a.get("message").getAsString() : a.toString()));
            return null;
        }
        DecisionLog.Answer answer = new DecisionLog.Answer();
        if (a.has("target") && !a.get("target").isJsonNull()) {
            String ref = a.get("target").getAsString();
            answer.target = DecisionLog.resolve(game, ref);
            if (answer.target == null) {
                stop("the answer " + ref + " names nothing in Java's game");
                return null;
            }
            // a Room's half or a card's face names the card; the cast dialog wants its spell
            mage.cards.Card card = question.startsWith("CHOOSE_ABILITY") ? game.getCard(answer.target) : null;
            if (card != null && card.getSpellAbility() != null) {
                answer.target = card.getSpellAbility().getId();
            }
        }
        if (a.has("bool") && !a.get("bool").isJsonNull()) {
            answer.bool = a.get("bool").getAsBoolean();
        }
        if (a.has("integer") && !a.get("integer").isJsonNull()) {
            answer.integer = a.get("integer").getAsInt();
        }
        if (a.has("string") && !a.get("string").isJsonNull()) {
            answer.string = a.get("string").getAsString();
        }
        if (a.has("manaType") && !a.get("manaType").isJsonNull()) {
            answer.manaType = ManaType.valueOf(a.get("manaType").getAsString());
        }
        // a choice the Rust copy doesn't list (a replacement effect's order, keyed by Java's ids): the first one
        mage.choices.Choice c = choice;
        if (answer.string != null && c != null && question.startsWith("CHOOSE_CHOICE")) {
            if (c.isKeyChoice() && !c.getKeyChoices().containsKey(answer.string) && !c.getKeyChoices().isEmpty()) {
                answer.string = c.getKeyChoices().keySet().iterator().next();
            } else if (!c.isKeyChoice() && !c.getChoices().contains(answer.string) && !c.getChoices().isEmpty()) {
                answer.string = c.getChoices().iterator().next();
            }
        }
        ANSWERED.incrementAndGet();
        return answer;
    }

    /** the other seat's answers since the last call */
    private void sendDecisions() {
        List<DecisionLog.Decision> decisions = log.getDecisions();
        for (; sent < decisions.size(); sent++) {
            DecisionLog.Decision d = decisions.get(sent);
            if (d.seat == seat) {
                continue;
            }
            JsonObject m = new JsonObject();
            m.addProperty("type", "decision");
            m.addProperty("seat", d.seat);
            if (d.target != null) m.addProperty("target", d.target);
            if (d.bool != null) m.addProperty("bool", d.bool);
            if (d.integer != null) m.addProperty("integer", d.integer);
            if (d.string != null) m.addProperty("string", d.string);
            if (d.manaType != null) m.addProperty("manaType", d.manaType);
            send(m.toString());
        }
    }

    private void send(String line) {
        if (!active) {
            return;
        }
        try {
            out.write(line);
            out.write('\n');
            out.flush();
        } catch (IOException e) {
            stop("the process went away: " + e.getMessage());
        }
    }

    private JsonObject next() {
        try {
            String line = lines.poll(ANSWER_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            if (line == null) {
                stop("no answer in " + ANSWER_TIMEOUT_MS / 1000 + " s");
                return null;
            }
            JsonObject o = JsonParser.parseString(line).getAsJsonObject();
            if ("closed".equals(type(o))) {
                stop("the process ended");
                return null;
            }
            return o;
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            stop("interrupted");
            return null;
        } catch (RuntimeException e) {
            stop("bad answer: " + e.getMessage());
            return null;
        }
    }

    private static String type(JsonObject o) {
        return o.has("type") ? o.get("type").getAsString() : "";
    }

    /** stops for good: the seat plays on with the Java AI */
    public void stop(String why) {
        boolean was = active || process == null;
        active = false;
        if (failure == null) {
            failure = why;
            if (was) {
                STOPPED.incrementAndGet();
                logger.warn("Rust AI stops in " + gameName + " (the Java AI plays on): " + why);
            }
        }
        close();
    }

    /** the game ended: the process is told and let go */
    public void end() {
        if (active) {
            send("{\"type\":\"end\"}");
            active = false;
            FINISHED.incrementAndGet();
        }
        close();
    }

    private void close() {
        if (process != null) {
            try {
                out.close();
            } catch (IOException ignore) {
            }
            Process p = process;
            Thread t = new Thread(() -> {
                try {
                    if (!p.waitFor(5, TimeUnit.SECONDS)) {
                        p.destroyForcibly();
                    }
                } catch (InterruptedException e) {
                    p.destroyForcibly();
                }
            });
            t.setDaemon(true);
            t.start();
        }
    }
}
