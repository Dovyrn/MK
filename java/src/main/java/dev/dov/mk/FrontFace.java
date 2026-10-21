package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum FrontFace {
    COUNTER_CLOCKWISE(0),
    CLOCKWISE(1);

    final int value;

    static FrontFace of(int v) {
        return switch (v) {
            case 0 -> COUNTER_CLOCKWISE;
            case 1 -> CLOCKWISE;
            default -> throw new IllegalArgumentException(v + " is no FrontFace");
        };
    }
}
