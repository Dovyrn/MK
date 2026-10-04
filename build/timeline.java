package dev.dov.mk;

public class Timeline {
    private long handle;

    Timeline(long handle) {
        this.handle = handle;
    }

    synchronized long handle() {
        if (handle == 0) {
            throw new IllegalStateException("timeline used after destroy");
        }
        return handle;
    }

    synchronized long take() {
        var taken = handle();
        handle = 0;
        return taken;
    }

    public synchronized long value() {
        return nValue(handle());
    }

    public synchronized boolean await(long value, long ms) {
        return nWait(handle(), value, ms);
    }

    public synchronized void signal(long value) {
        nSignal(handle(), value);
    }

    public synchronized long raw() {
        return nRaw(handle());
    }

    private static native long nValue(long timeline);

    private static native boolean nWait(long timeline, long value, long ms);

    private static native void nSignal(long timeline, long value);

    private static native long nRaw(long timeline);
}
