package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum Filter {
    NEAREST(0),
    LINEAR(1);

    final int value;

    static Filter of(int v) {
        return switch (v) {
            case 0 -> NEAREST;
            case 1 -> LINEAR;
            default -> throw new IllegalArgumentException(v + " is no Filter");
        };
    }
}
