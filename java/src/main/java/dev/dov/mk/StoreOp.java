package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum StoreOp {
    STORE(0),
    DISCARD(1);

    final int value;

    static StoreOp of(int v) {
        return switch (v) {
            case 0 -> STORE;
            case 1 -> DISCARD;
            default -> throw new IllegalArgumentException(v + " is no StoreOp");
        };
    }
}
