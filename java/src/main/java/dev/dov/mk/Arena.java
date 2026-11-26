package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.util.ArrayList;
import java.util.concurrent.locks.Lock;

class Arena implements AutoCloseable {
    private static final long START = 4096;
    private static final long ALIGN = 16;
    private static final ThreadLocal<Arena> LOCAL = ThreadLocal.withInitial(() -> new Arena(true));

    private final ArrayList<Long> spills = new ArrayList<>();
    private final ArrayList<Lock> locks = new ArrayList<>();
    private final boolean shared;
    private long block;
    private long size;
    private long used;
    private long wanted;
    private boolean open;

    private Arena(boolean shared) {
        this.shared = shared;
    }

    static Arena open() {
        var arena = LOCAL.get();
        if (arena.open) {
            return new Arena(false);
        }
        arena.open = true;
        return arena;
    }

    void unlockOnClose(Lock lock) {
        locks.add(lock);
    }

    long alloc(long bytes) {
        if (bytes == 0) {
            return NULL;
        }
        var at = (used + ALIGN - 1) & -ALIGN;
        if (at + bytes <= size) {
            used = at + bytes;
            return block + at;
        }
        var address = nmemAlloc(bytes);
        if (address == NULL) {
            throw new OutOfMemoryError(bytes + " bytes of call arguments");
        }
        wanted += bytes + ALIGN;
        spills.add(address);
        return address;
    }

    long ints(int[] values) {
        var address = alloc(values.length * 4L);
        for (var i = 0; i < values.length; i++) {
            memPutInt(address + i * 4L, values[i]);
        }
        return address;
    }

    long bytes(byte[] values) {
        var address = alloc(values.length);
        for (var i = 0; i < values.length; i++) {
            memPutByte(address + i, values[i]);
        }
        return address;
    }

    long floats(float[] values) {
        var address = alloc(values.length * 4L);
        for (var i = 0; i < values.length; i++) {
            memPutFloat(address + i * 4L, values[i]);
        }
        return address;
    }

    @Override
    public void close() {
        for (var i = 0; i < spills.size(); i++) {
            nmemFree(spills.get(i));
        }
        spills.clear();
        for (var i = 0; i < locks.size(); i++) {
            locks.get(i).unlock();
        }
        locks.clear();
        if (!shared) {
            return;
        }
        if (wanted > 0) {
            nmemFree(block);
            size = Math.max(START, Long.highestOneBit(used + wanted) << 1);
            block = nmemAlloc(size);
            if (block == NULL) {
                size = 0;
            }
            wanted = 0;
        }
        used = 0;
        open = false;
    }
}
