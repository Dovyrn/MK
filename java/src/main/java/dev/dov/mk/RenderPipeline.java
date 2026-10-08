package dev.dov.mk;

public record RenderPipeline(long raw) {
    static RenderPipeline of(long raw) {
        return raw == 0 ? null : new RenderPipeline(raw);
    }

    static long raw(RenderPipeline handle) {
        return handle == null ? 0 : handle.raw;
    }
}
