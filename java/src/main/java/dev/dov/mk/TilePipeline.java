package dev.dov.mk;

public record TilePipeline(long raw) {
    static TilePipeline of(long raw) {
        return raw == 0 ? null : new TilePipeline(raw);
    }

    static long raw(TilePipeline handle) {
        return handle == null ? 0 : handle.raw;
    }
}
