public class Commands implements AutoCloseable {
    private static final Cleaner cleaner = Cleaner.create();
    private final Block block = new Block();
    private final Cleaner.Cleanable cleanable = cleaner.register(this, block);
    private int capacity = 4096;
    @Getter
    private int size;

    public long getAddress() {
        return block.address;
    }

    public void reset() {
        size = 0;
    }

    @Override
    public void close() {
        cleanable.clean();
    }

    protected long head(int op, int length) {
        if (block.address == 0) {
            throw new IllegalStateException("commands used after close");
        }
        if (length < 0 || length > 0xffff) {
            throw new IllegalArgumentException("payload of " + length + " bytes is outside 0..65535");
        }
        var needed = (long) size + 4 + length;
        if (needed > Integer.MAX_VALUE) {
            throw new IllegalStateException("commands are over 2 GiB");
        }
        if (needed > capacity) {
            capacity = (int) Math.min(Integer.MAX_VALUE, Math.max(2L * capacity, needed));
            var grown = nmemRealloc(block.address, capacity);
            if (grown == NULL) {
                throw new OutOfMemoryError("command stream of " + capacity + " bytes");
            }
            block.address = grown;
        }
        var p = block.address + size;
        memPutShort(p, (short) op);
        memPutShort(p + 2, (short) length);
        size += 4 + length;
        return p + 4;
    }
