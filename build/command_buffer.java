public class CommandBuffer implements AutoCloseable {
    final Device device;
    private long handle;
    private boolean passing;

    CommandBuffer(Device device, long handle) {
        this.device = device;
        this.handle = handle;
        device.recording++;
    }

    static CommandBuffer adopt(Device device, long handle) {
        return new CommandBuffer(device, handle);
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("command buffer used after submit, finish or close");
        }
        if (passing) {
            throw new IllegalStateException("command buffer used while one of its passes is open; end the pass first");
        }
        return handle;
    }

    long take() {
        var taken = handle();
        handle = 0;
        device.recording--;
        return taken;
    }

    void ended() {
        passing = false;
    }

    public ComputePass beginComputePass() {
        synchronized (device) {
            var pass = nBeginComputePass(handle());
            passing = true;
            return new ComputePass(device, this, pass);
        }
    }

    public RenderPass beginRenderPass(RenderPassInfo info) {
        synchronized (device) {
            try (var arena = new Arena()) {
                var size = info.encodedSize();
                var address = arena.alloc(size);
                info.encode(address);
                var pass = nBeginRenderPass(handle(), address, size);
                passing = true;
                return new RenderPass(device, this, pass);
            }
        }
    }

    public void finish() {
        synchronized (device) {
            nFinish(take());
        }
    }

    public long raw() {
        synchronized (device) {
            return nRaw(handle());
        }
    }

    @Override
    public void close() {
        synchronized (device) {
            if (handle != 0) {
                var dropped = handle;
                handle = 0;
                device.recording--;
                nDrop(dropped);
            }
        }
    }

    private static native long nBeginComputePass(long commands);

    private static native long nBeginRenderPass(long commands, long info, int infoLength);

    private static native void nFinish(long commands);

    private static native long nRaw(long commands);

    private static native void nDrop(long commands);
