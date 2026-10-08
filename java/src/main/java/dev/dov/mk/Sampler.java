package dev.dov.mk;

public record Sampler(long raw) {
    static Sampler of(long raw) {
        return raw == 0 ? null : new Sampler(raw);
    }

    static long raw(Sampler handle) {
        return handle == null ? 0 : handle.raw;
    }
}
