package dev.dov.mk;

public record Texture(long raw) {
    static Texture of(long raw) {
        return raw == 0 ? null : new Texture(raw);
    }

    static long raw(Texture handle) {
        return handle == null ? 0 : handle.raw;
    }
}
