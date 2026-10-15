package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.util.ArrayList;
import java.util.concurrent.locks.Lock;

class Arena implements AutoCloseable {
    private final ArrayList<Long> blocks = new ArrayList<>();
    private final ArrayList<Lock> locks = new ArrayList<>();

    void unlockOnClose(Lock lock) {
        locks.add(lock);
    }

    long alloc(long size) {
        if (size == 0) {
            return NULL;
        }
        var address = nmemAlloc(size);
        if (address == NULL) {
            throw new OutOfMemoryError(size + " bytes of call arguments");
        }
        blocks.add(address);
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
        for (var address : blocks) {
            nmemFree(address);
        }
        for (var lock : locks) {
            lock.unlock();
        }
    }
}
