package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum BorderColor {
    TRANSPARENT_BLACK(0),
    OPAQUE_BLACK(1),
    OPAQUE_WHITE(2);

    final int value;

    static BorderColor of(int v) {
        return switch (v) {
            case 0 -> TRANSPARENT_BLACK;
            case 1 -> OPAQUE_BLACK;
            case 2 -> OPAQUE_WHITE;
            default -> throw new IllegalArgumentException(v + " is no BorderColor");
        };
    }
}
