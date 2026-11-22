package mage.server;

import org.junit.jupiter.api.Test;

import java.io.File;

import static org.junit.jupiter.api.Assertions.*;

/** Low free disk under db/: the floor is the larger of a fixed size and a share of the disk. */
public class DiskStatusTest {

    private static final long GB = 1L << 30;

    @Test
    public void plentyIsFine() {
        DiskStatus.Disk disk = DiskStatus.describe(24 * GB, 75 * GB, 3 * GB);
        assertNull(disk.problem);
        assertEquals(24 * GB, disk.free);
    }

    @Test
    public void underTheFixedFloorIsLow() {
        // 5% of 40 GB is 2 GB: the 3 GB floor wins
        DiskStatus.Disk disk = DiskStatus.describe((long) (2.5 * GB), 40 * GB, 3 * GB);
        assertNotNull(disk.problem);
        assertTrue(disk.problem.contains("nearly full"), disk.problem);
    }

    @Test
    public void underFivePercentOfABigDiskIsLow() {
        // 5% of 500 GB is 25 GB, more than the 3 GB floor
        assertNotNull(DiskStatus.describe(20 * GB, 500 * GB, 3 * GB).problem);
        assertNull(DiskStatus.describe(30 * GB, 500 * GB, 3 * GB).problem);
    }

    @Test
    public void readsARealDirectory() {
        DiskStatus.Disk disk = DiskStatus.read(new File("."), 0);
        assertNotNull(disk);
        assertTrue(disk.total > 0 && disk.free > 0);
    }
}
