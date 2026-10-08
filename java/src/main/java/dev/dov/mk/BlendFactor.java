package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum BlendFactor {
    ZERO(0),
    ONE(1),
    SRC_COLOR(2),
    ONE_MINUS_SRC_COLOR(3),
    DST_COLOR(4),
    ONE_MINUS_DST_COLOR(5),
    SRC_ALPHA(6),
    ONE_MINUS_SRC_ALPHA(7),
    DST_ALPHA(8),
    ONE_MINUS_DST_ALPHA(9),
    CONSTANT_COLOR(10),
    ONE_MINUS_CONSTANT_COLOR(11),
    CONSTANT_ALPHA(12),
    ONE_MINUS_CONSTANT_ALPHA(13),
    SRC_ALPHA_SATURATE(14),
    SRC1_COLOR(15),
    ONE_MINUS_SRC1_COLOR(16),
    SRC1_ALPHA(17),
    ONE_MINUS_SRC1_ALPHA(18);

    final int value;

    static BlendFactor of(int v) {
        return switch (v) {
            case 0 -> ZERO;
            case 1 -> ONE;
            case 2 -> SRC_COLOR;
            case 3 -> ONE_MINUS_SRC_COLOR;
            case 4 -> DST_COLOR;
            case 5 -> ONE_MINUS_DST_COLOR;
            case 6 -> SRC_ALPHA;
            case 7 -> ONE_MINUS_SRC_ALPHA;
            case 8 -> DST_ALPHA;
            case 9 -> ONE_MINUS_DST_ALPHA;
            case 10 -> CONSTANT_COLOR;
            case 11 -> ONE_MINUS_CONSTANT_COLOR;
            case 12 -> CONSTANT_ALPHA;
            case 13 -> ONE_MINUS_CONSTANT_ALPHA;
            case 14 -> SRC_ALPHA_SATURATE;
            case 15 -> SRC1_COLOR;
            case 16 -> ONE_MINUS_SRC1_COLOR;
            case 17 -> SRC1_ALPHA;
            case 18 -> ONE_MINUS_SRC1_ALPHA;
            default -> throw new IllegalArgumentException(v + " is no BlendFactor");
        };
    }
}
