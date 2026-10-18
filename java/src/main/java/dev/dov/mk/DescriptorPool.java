package dev.dov.mk;

public record DescriptorPool(long raw) {
    static DescriptorPool of(long raw) {
        return raw == 0 ? null : new DescriptorPool(raw);
    }

    static long raw(DescriptorPool handle) {
        return handle == null ? 0 : handle.raw;
    }
}
