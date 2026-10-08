package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum IndexType {
    U16(0),
    U32(1);

    final int value;

    static IndexType of(int v) {
        return switch (v) {
            case 0 -> U16;
            case 1 -> U32;
            default -> throw new IllegalArgumentException(v + " is no IndexType");
        };
    }
}
