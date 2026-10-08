package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum FrontFace {
    CCW(0),
    CW(1);

    final int value;

    static FrontFace of(int v) {
        return switch (v) {
            case 0 -> CCW;
            case 1 -> CW;
            default -> throw new IllegalArgumentException(v + " is no FrontFace");
        };
    }
}
