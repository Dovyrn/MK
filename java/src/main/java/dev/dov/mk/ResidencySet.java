package dev.dov.mk;

public record ResidencySet(long raw) {
    static ResidencySet of(long raw) {
        return raw == 0 ? null : new ResidencySet(raw);
    }

    static long raw(ResidencySet handle) {
        return handle == null ? 0 : handle.raw;
    }
}
