package mage.util;

import java.awt.*;
import java.util.Collection;
import java.util.Random;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.Callable;

/**
 * Random numbers for the engine. A thread running a game uses that game's own generator (installed with
 * {@link #install(Random)}, see {@link GameRandom}), so a game started from a seed shuffles, mulligans and flips coins
 * the same way every time. Any other thread shares one server-wide generator.
 * <p>
 * Created by IGOUDT on 5-9-2016.
 */
public final class RandomUtil {

    private static final Random random = new Random(); // thread safe with seed support
    private static final ThreadLocal<Random> scoped = new ThreadLocal<>();

    private RandomUtil() {
    }

    /**
     * @return the generator for this thread: the game's own while a game runs on it, otherwise the shared one
     */
    public static Random getRandom() {
        Random own = scoped.get();
        return own != null ? own : random;
    }

    /**
     * Use this generator for every random number on the current thread until {@link #uninstall()}.
     *
     * @return the generator that was installed before, to put back afterwards (null for none)
     */
    public static Random install(Random generator) {
        Random before = scoped.get();
        if (generator == null) {
            scoped.remove();
        } else {
            scoped.set(generator);
        }
        return before;
    }

    public static void uninstall() {
        scoped.remove();
    }

    /**
     * @return true when the current thread draws from a game's own generator
     */
    public static boolean isInstalled() {
        return scoped.get() != null;
    }

    /**
     * A generator for work this thread hands to another one (an AI search): drawn from the game's generator when one
     * is installed, so the work is reproducible too; null when there is none and the other thread should use the
     * shared one.
     */
    public static Random childForOtherThread() {
        Random own = scoped.get();
        if (own instanceof GameRandom) {
            return ((GameRandom) own).child();
        }
        return own == null ? null : new GameRandom(own.nextLong());
    }

    /**
     * Runs {@code work} with {@code generator} installed on the current thread, then puts back what was there.
     */
    public static <T> T withRandom(Random generator, Callable<T> work) throws Exception {
        Random before = install(generator);
        try {
            return work.call();
        } finally {
            install(before);
        }
    }

    public static int nextInt() {
        return getRandom().nextInt();
    }

    public static int nextInt(int max) {
        return getRandom().nextInt(max);
    }

    public static boolean nextBoolean() {
        return getRandom().nextBoolean();
    }

    public static double nextDouble() {
        return getRandom().nextDouble();
    }

    public static Color nextColor() {
        return new Color(RandomUtil.nextInt(256), RandomUtil.nextInt(256), RandomUtil.nextInt(256));
    }

    /**
     * Seeds the generator the current thread uses (the game's own while a game runs on it).
     */
    public static void setSeed(long newSeed) {
        getRandom().setSeed(newSeed);
    }

    public static <T> T randomFromCollection(Collection<T> collection) {
        if (collection.size() < 2) {
            return collection.stream().findFirst().orElse(null);
        }
        int rand = nextInt(collection.size());
        int count = 0;
        for (T current : collection) {
            if (count == rand) {
                return current;
            }
            count++;
        }
        return null;
    }
}
