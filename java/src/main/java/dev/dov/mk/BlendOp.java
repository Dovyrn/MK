package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum BlendOp {
    ADD(0),
    SUBTRACT(1),
    REVERSE_SUBTRACT(2),
    MIN(3),
    MAX(4);

    final int value;

    static BlendOp of(int v) {
        return switch (v) {
            case 0 -> ADD;
            case 1 -> SUBTRACT;
            case 2 -> REVERSE_SUBTRACT;
            case 3 -> MIN;
            case 4 -> MAX;
            default -> throw new IllegalArgumentException(v + " is no BlendOp");
        };
    }
}
