package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum ResourceKind {
    BUFFER(0),
    TEXTURE(1),
    QUERY_POOL(2),
    INDIRECT_BUFFER(3);

    final int value;

    static ResourceKind of(int v) {
        return switch (v) {
            case 0 -> BUFFER;
            case 1 -> TEXTURE;
            case 2 -> QUERY_POOL;
            case 3 -> INDIRECT_BUFFER;
            default -> throw new IllegalArgumentException(v + " is no ResourceKind");
        };
    }
}
