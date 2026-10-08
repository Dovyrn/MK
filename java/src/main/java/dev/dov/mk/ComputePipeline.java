package dev.dov.mk;

public record ComputePipeline(long raw) {
    static ComputePipeline of(long raw) {
        return raw == 0 ? null : new ComputePipeline(raw);
    }

    static long raw(ComputePipeline handle) {
        return handle == null ? 0 : handle.raw;
    }
}
