package mage.server.ladder;

import org.apache.log4j.Logger;

import java.time.Instant;
import java.time.LocalDate;
import java.time.ZoneOffset;
import java.time.format.DateTimeParseException;

/**
 * The ladder's seasons (#434): the same calendar for every format and queue, `months` long from `start` (UTC). Seasons
 * are counted from 0, shown as "Season 1"; the ladder's tables keep the index. A server sets the calendar with
 * xmage.seasonStart / XMAGE_SEASON_START (a date, 2026-11-01 by default) and xmage.seasonMonths / XMAGE_SEASON_MONTHS
 * (2 by default). Changing it never takes a season back: {@link RankedService} keeps the highest season already played.
 */
public final class Seasons {

    private static final Logger logger = Logger.getLogger(Seasons.class);
    public static final LocalDate DEFAULT_START = LocalDate.of(2026, 11, 1);
    public static final int DEFAULT_MONTHS = 2;

    private static volatile Seasons instance;

    private final LocalDate start;
    private final int months;

    public Seasons(LocalDate start, int months) {
        if (months < 1 || months > 24) {
            throw new IllegalArgumentException("A season is 1 to 24 months long, not " + months);
        }
        this.start = start;
        this.months = months;
    }

    public static Seasons get() {
        Seasons current = instance;
        if (current == null) {
            current = fromSettings(setting("xmage.seasonStart", "XMAGE_SEASON_START"), setting("xmage.seasonMonths", "XMAGE_SEASON_MONTHS"));
            instance = current;
        }
        return current;
    }

    /** for tests */
    public static void use(Seasons seasons) {
        instance = seasons;
    }

    static Seasons fromSettings(String startText, String monthsText) {
        LocalDate start = DEFAULT_START;
        int months = DEFAULT_MONTHS;
        try {
            if (startText != null && !startText.trim().isEmpty()) {
                start = LocalDate.parse(startText.trim());
            }
            if (monthsText != null && !monthsText.trim().isEmpty()) {
                months = Integer.parseInt(monthsText.trim());
            }
            return new Seasons(start, months);
        } catch (DateTimeParseException | IllegalArgumentException e) {
            logger.error("Ladder seasons: the season settings (" + startText + ", " + monthsText + ") aren't valid, using "
                    + DEFAULT_START + " and " + DEFAULT_MONTHS + " months", e);
            return new Seasons(DEFAULT_START, DEFAULT_MONTHS);
        }
    }

    private static String setting(String property, String env) {
        String value = System.getProperty(property);
        return value != null ? value : System.getenv(env);
    }

    public LocalDate start() {
        return start;
    }

    public int months() {
        return months;
    }

    /** the season `now` falls in by the calendar; 0 before the first one starts */
    public int index(long now) {
        LocalDate day = Instant.ofEpochMilli(now).atZone(ZoneOffset.UTC).toLocalDate();
        if (day.isBefore(start)) {
            return 0;
        }
        int index = (int) (java.time.temporal.ChronoUnit.MONTHS.between(start, day) / months);
        // MONTHS.between counts whole months, so the first day of a season is in it
        return Math.max(0, index);
    }

    /** when a season starts, ms UTC */
    public long startsAt(int season) {
        return start.plusMonths((long) season * months).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli();
    }

    /** when a season ends (the next one's start), ms UTC */
    public long endsAt(int season) {
        return startsAt(season + 1);
    }

    /** "Season 1" for index 0 */
    public static String name(int season) {
        return "Season " + (season + 1);
    }
}
