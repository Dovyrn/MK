package dev.dov.mk;

public record InputAttachments(long raw) {
    static InputAttachments of(long raw) {
        return raw == 0 ? null : new InputAttachments(raw);
    }

    static long raw(InputAttachments handle) {
        return handle == null ? 0 : handle.raw;
    }
}
