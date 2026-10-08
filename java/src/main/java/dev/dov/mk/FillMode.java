package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum FillMode {
    FILL(0),
    LINE(1);

    final int value;

    static FillMode of(int v) {
        return switch (v) {
            case 0 -> FILL;
            case 1 -> LINE;
            default -> throw new IllegalArgumentException(v + " is no FillMode");
        };
    }
}
