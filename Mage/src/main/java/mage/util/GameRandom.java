package mage.util;

import java.util.Random;

/**
 * A game's own random numbers: the same linear congruential generator as {@link Random}, with its state in a plain
 * field, so a game state can copy it, save it to disk and put it back exactly. Two games started from the same seed
 * draw the same numbers in the same order.
 * <p>
 * Not thread safe: one game thread uses it at a time (see {@link RandomUtil#install(Random)}).
 */
public final class GameRandom extends Random {

    private static final long serialVersionUID = 1L;

    // the constants of java.util.Random
    private static final long MULTIPLIER = 0x5DEECE66DL;
    private static final long ADDEND = 0xBL;
    private static final long MASK = (1L << 48) - 1;

    private long seed;
    private long state;
    private boolean ready; // Random's constructor calls setSeed before this class's fields exist

    public GameRandom(long seed) {
        super(0L);
        this.seed = seed;
        this.state = scramble(seed);
        this.ready = true;
    }

    private GameRandom(long seed, long state) {
        super(0L);
        this.seed = seed;
        this.state = state;
        this.ready = true;
    }

    private static long scramble(long seed) {
        return (seed ^ MULTIPLIER) & MASK;
    }

    /**
     * @return the seed the generator started from (setSeed doesn't change it)
     */
    public long getSeed() {
        return seed;
    }

    /**
     * @return the generator's position, for {@link #setState(long)}
     */
    public long getState() {
        return state;
    }

    public void setState(long state) {
        this.state = state & MASK;
    }

    /**
     * Moves this generator to exactly where {@code other} is (seed and position), so whoever holds this one carries
     * on from there.
     */
    public void restoreFrom(GameRandom other) {
        this.seed = other.seed;
        this.state = other.state;
    }

    public GameRandom copy() {
        return new GameRandom(seed, state);
    }

    /**
     * A new generator for work that runs beside the game (an AI search on its own thread): it draws one number from
     * this one, so the child is reproducible too, and nothing the child draws moves this one.
     */
    public GameRandom child() {
        return new GameRandom(nextLong());
    }

    @Override
    public synchronized void setSeed(long seed) {
        if (ready) {
            this.state = scramble(seed);
        }
        super.setSeed(seed); // clears Random's cached nextGaussian
    }

    @Override
    protected int next(int bits) {
        state = (state * MULTIPLIER + ADDEND) & MASK;
        return (int) (state >>> (48 - bits));
    }
}
