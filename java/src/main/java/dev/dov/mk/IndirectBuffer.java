package dev.dov.mk;

public record IndirectBuffer(long raw) {
    static IndirectBuffer of(long raw) {
        return raw == 0 ? null : new IndirectBuffer(raw);
    }

    static long raw(IndirectBuffer handle) {
        return handle == null ? 0 : handle.raw;
    }

    public Resource resource() {
        return new Resource().kind(ResourceKind.INDIRECT_BUFFER).handle(raw);
    }
}
