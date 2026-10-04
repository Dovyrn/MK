public class Device implements AutoCloseable {
    protected long handle;

    public Device(long handle) {
        this.handle = handle;
    }

    @Override
    public synchronized void close() {
        var device = handle;
        handle = 0;
        if (device != 0) {
            nFree(device);
        }
    }

    private static native void nFree(long device);
