package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum CompareOp {
    NONE(0),
    NEVER(1),
    LESS(2),
    EQUAL(3),
    LESS_EQUAL(4),
    GREATER(5),
    NOT_EQUAL(6),
    GREATER_EQUAL(7),
    ALWAYS(8);

    final int value;

    static CompareOp of(int v) {
        return switch (v) {
            case 0 -> NONE;
            case 1 -> NEVER;
            case 2 -> LESS;
            case 3 -> EQUAL;
            case 4 -> LESS_EQUAL;
            case 5 -> GREATER;
            case 6 -> NOT_EQUAL;
            case 7 -> GREATER_EQUAL;
            case 8 -> ALWAYS;
            default -> throw new IllegalArgumentException(v + " is no CompareOp");
        };
    }
}
