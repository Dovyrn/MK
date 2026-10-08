package dev.dov.mk;

public record CommandPool(long raw) {
    static CommandPool of(long raw) {
        return raw == 0 ? null : new CommandPool(raw);
    }

    static long raw(CommandPool handle) {
        return handle == null ? 0 : handle.raw;
    }
}
