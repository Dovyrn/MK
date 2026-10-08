package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum TextureLayout {
    UNDEFINED(0),
    GENERAL(1),
    READ(2),
    COLOR(3),
    DEPTH(4),
    DEPTH_READ(5),
    COPY_SRC(6),
    COPY_DST(7),
    PRESENT(8);

    final int value;

    static TextureLayout of(int v) {
        return switch (v) {
            case 0 -> UNDEFINED;
            case 1 -> GENERAL;
            case 2 -> READ;
            case 3 -> COLOR;
            case 4 -> DEPTH;
            case 5 -> DEPTH_READ;
            case 6 -> COPY_SRC;
            case 7 -> COPY_DST;
            case 8 -> PRESENT;
            default -> throw new IllegalArgumentException(v + " is no TextureLayout");
        };
    }
}
