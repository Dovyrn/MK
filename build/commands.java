@Getter
public class Commands implements AutoCloseable {
    private long address = nmemAlloc(4096);
    private int capacity = 4096;
    private int size;

    public void reset() {
        size = 0;
    }

    @Override
    public void close() {
        nmemFree(address);
        address = 0;
    }

    protected long head(int op, int length) {
        if (address == 0) {
            throw new IllegalStateException("commands used after close");
        }
        if (length > 0xffff) {
            throw new IllegalArgumentException("payload of " + length + " bytes is over 65535");
        }
        if (size + 4 + length > capacity) {
            capacity = Math.max(capacity * 2, size + 4 + length);
            address = nmemRealloc(address, capacity);
        }
        var p = address + size;
        memPutShort(p, (short) op);
        memPutShort(p + 2, (short) length);
        size += 4 + length;
        return p + 4;
    }
