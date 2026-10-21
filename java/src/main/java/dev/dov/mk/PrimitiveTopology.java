package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum PrimitiveTopology {
    POINT(0),
    LINE(1),
    LINE_STRIP(2),
    TRIANGLE(3),
    TRIANGLE_STRIP(4);

    final int value;

    static PrimitiveTopology of(int v) {
        return switch (v) {
            case 0 -> POINT;
            case 1 -> LINE;
            case 2 -> LINE_STRIP;
            case 3 -> TRIANGLE;
            case 4 -> TRIANGLE_STRIP;
            default -> throw new IllegalArgumentException(v + " is no PrimitiveTopology");
        };
    }
}
