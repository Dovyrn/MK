package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum LoadOp {
    LOAD(0),
    CLEAR(1),
    DONT_CARE(2);

    final int value;

    static LoadOp of(int v) {
        return switch (v) {
            case 0 -> LOAD;
            case 1 -> CLEAR;
            case 2 -> DONT_CARE;
            default -> throw new IllegalArgumentException(v + " is no LoadOp");
        };
    }
}
