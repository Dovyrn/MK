package dev.dov.mk;

public record DepthState(long raw) {
    static DepthState of(long raw) {
        return raw == 0 ? null : new DepthState(raw);
    }

    static long raw(DepthState handle) {
        return handle == null ? 0 : handle.raw;
    }
}
