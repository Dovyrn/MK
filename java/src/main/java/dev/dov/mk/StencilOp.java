package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum StencilOp {
    KEEP(0),
    ZERO(1),
    REPLACE(2),
    INCREMENT_CLAMP(3),
    DECREMENT_CLAMP(4),
    INVERT(5),
    INCREMENT_WRAP(6),
    DECREMENT_WRAP(7);

    final int value;

    static StencilOp of(int v) {
        return switch (v) {
            case 0 -> KEEP;
            case 1 -> ZERO;
            case 2 -> REPLACE;
            case 3 -> INCREMENT_CLAMP;
            case 4 -> DECREMENT_CLAMP;
            case 5 -> INVERT;
            case 6 -> INCREMENT_WRAP;
            case 7 -> DECREMENT_WRAP;
            default -> throw new IllegalArgumentException(v + " is no StencilOp");
        };
    }
}
