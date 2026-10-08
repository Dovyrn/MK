public class NAME implements AutoCloseable {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    NAME(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording++;
        }
    }

    static NAME adopt(Device device, long handle) {
        return new NAME(device, null, handle);
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("pass used after end");
        }
        return handle;
    }

    public void end() {
        synchronized (device) {
            var ended = handle();
            handle = 0;
            if (owner != null) {
                owner.ended();
            } else {
                device.recording--;
            }
            nEnd(ended);
        }
    }

    public long raw() {
        synchronized (device) {
            return nRaw(handle());
        }
    }

    @Override
    public void close() {
        if (handle != 0) {
            end();
        }
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);
