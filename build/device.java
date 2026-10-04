public class Device implements AutoCloseable {
    protected long handle;

    public Device(long handle) {
        this.handle = handle;
    }

    @Override
    public synchronized void close() {
        if (handle != 0) {
            nFree(handle);
            handle = 0;
        }
    }

    private static native void nFree(long device);
