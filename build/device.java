public class Device implements AutoCloseable {
    protected long handle;
    final AtomicInteger recording = new AtomicInteger();
    final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

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
        var count = recording.get();
        if (count > 0) {
            throw new IllegalStateException(call + " changes the device while " + count
                    + " command buffers or imported passes are recording; submit or close them first");
        }
    }

    @Override
    public void close() {
        lock.writeLock().lock();
        try {
            idle("close");
            var device = handle;
            handle = 0;
            if (device != 0) {
                nFree(device);
            }
        } finally {
            lock.writeLock().unlock();
        }
    }

    private static native void nFree(long device);
