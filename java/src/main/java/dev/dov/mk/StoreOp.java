package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum StoreOp {
    STORE(0),
    DONT_CARE(1);

    final int value;

    static StoreOp of(int v) {
        return switch (v) {
            case 0 -> STORE;
            case 1 -> DONT_CARE;
            default -> throw new IllegalArgumentException(v + " is no StoreOp");
        };
    }
}
