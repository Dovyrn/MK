package dev.dov.mk;

public record PipelineCache(long raw) {
    static PipelineCache of(long raw) {
        return raw == 0 ? null : new PipelineCache(raw);
    }

    static long raw(PipelineCache handle) {
        return handle == null ? 0 : handle.raw;
    }
}
