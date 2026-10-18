package dev.dov.mk;

public record DescriptorSet(long raw) {
    static DescriptorSet of(long raw) {
        return raw == 0 ? null : new DescriptorSet(raw);
    }

    static long raw(DescriptorSet handle) {
        return handle == null ? 0 : handle.raw;
    }
}
