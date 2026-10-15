package dev.dov.mk;

public record Swapchain(long raw) {
    static Swapchain of(long raw) {
        return raw == 0 ? null : new Swapchain(raw);
    }

    static long raw(Swapchain handle) {
        return handle == null ? 0 : handle.raw;
    }
}
