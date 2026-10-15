package dev.dov.mk;

import java.util.concurrent.locks.ReentrantReadWriteLock;

public class Timeline {
    private volatile long handle;
    private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    Timeline(long handle) {
        this.handle = handle;
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("timeline used after destroy");
        }
        return handle;
    }

    long handle(Arena arena) {
        var read = lock.readLock();
        read.lock();
        arena.unlockOnClose(read);
        return handle();
    }

    long take() {
        lock.writeLock().lock();
        try {
            var taken = handle();
            handle = 0;
            return taken;
        } finally {
            lock.writeLock().unlock();
        }
    }

    public long value() {
        lock.readLock().lock();
        try {
            return nValue(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    public boolean await(long value, long ms) {
        lock.readLock().lock();
        try {
            return nWait(handle(), value, ms);
        } finally {
            lock.readLock().unlock();
        }
    }

    public void signal(long value) {
        lock.readLock().lock();
        try {
            nSignal(handle(), value);
        } finally {
            lock.readLock().unlock();
        }
    }

    public long raw() {
        lock.readLock().lock();
        try {
            return nRaw(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nValue(long timeline);

    private static native boolean nWait(long timeline, long value, long ms);

    private static native void nSignal(long timeline, long value);

    private static native long nRaw(long timeline);
}
