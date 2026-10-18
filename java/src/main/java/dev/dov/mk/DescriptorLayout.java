package dev.dov.mk;

public record DescriptorLayout(long raw) {
    static DescriptorLayout of(long raw) {
        return raw == 0 ? null : new DescriptorLayout(raw);
    }

    static long raw(DescriptorLayout handle) {
        return handle == null ? 0 : handle.raw;
    }
}
