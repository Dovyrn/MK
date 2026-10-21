package dev.dov.mk;

public record Buffer(long raw) {
    static Buffer of(long raw) {
        return raw == 0 ? null : new Buffer(raw);
    }

    static long raw(Buffer handle) {
        return handle == null ? 0 : handle.raw;
    }

    public ResidentResource resource() {
        return new ResidentResource().kind(ResourceKind.BUFFER).handle(raw);
    }
}
