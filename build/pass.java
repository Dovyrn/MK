public class NAME {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    NAME(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording.incrementAndGet();
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
        var ended = handle();
        handle = 0;
        nEnd(ended);
        if (owner != null) {
            owner.ended();
        } else {
            device.recording.decrementAndGet();
        }
    }

    public long raw() {
        return nRaw(handle());
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);
