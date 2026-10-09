public class Device implements AutoCloseable {
    protected long handle;
    int recording;

    public Device(long handle) {
        this.handle = handle;
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("device used after close");
        }
        return handle;
    }

    void idle(String call) {
        if (recording > 0) {
            throw new IllegalStateException(call + " changes the device while " + recording
                    + " command buffers or imported passes are recording; submit or close them first");
        }
    }

    @Override
    public synchronized void close() {
        idle("close");
        var device = handle;
        handle = 0;
        if (device != 0) {
            nFree(device);
        }
    }

    private static native void nFree(long device);
