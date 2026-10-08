package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum TextureDimension {
    D1(0),
    D2(1),
    D3(2),
    CUBE(3),
    D1_ARRAY(4),
    D2_ARRAY(5),
    CUBE_ARRAY(6);

    final int value;

    static TextureDimension of(int v) {
        return switch (v) {
            case 0 -> D1;
            case 1 -> D2;
            case 2 -> D3;
            case 3 -> CUBE;
            case 4 -> D1_ARRAY;
            case 5 -> D2_ARRAY;
            case 6 -> CUBE_ARRAY;
            default -> throw new IllegalArgumentException(v + " is no TextureDimension");
        };
    }
}
