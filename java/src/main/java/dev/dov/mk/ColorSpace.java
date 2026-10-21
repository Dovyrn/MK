package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum ColorSpace {
    SRGB(0),
    DISPLAY_P3(1),
    EXTENDED_LINEAR_SRGB(2),
    HDR10(3);

    final int value;

    static ColorSpace of(int v) {
        return switch (v) {
            case 0 -> SRGB;
            case 1 -> DISPLAY_P3;
            case 2 -> EXTENDED_LINEAR_SRGB;
            case 3 -> HDR10;
            default -> throw new IllegalArgumentException(v + " is no ColorSpace");
        };
    }
}
