package dev.dov.mk;

public record DepthStencilState(long raw) {
    static DepthStencilState of(long raw) {
        return raw == 0 ? null : new DepthStencilState(raw);
    }

    static long raw(DepthStencilState handle) {
        return handle == null ? 0 : handle.raw;
    }
}
