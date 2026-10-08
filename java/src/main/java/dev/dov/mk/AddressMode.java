package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum AddressMode {
    REPEAT(0),
    MIRROR(1),
    CLAMP(2),
    BORDER(3),
    MIRROR_CLAMP(4);

    final int value;

    static AddressMode of(int v) {
        return switch (v) {
            case 0 -> REPEAT;
            case 1 -> MIRROR;
            case 2 -> CLAMP;
            case 3 -> BORDER;
            case 4 -> MIRROR_CLAMP;
            default -> throw new IllegalArgumentException(v + " is no AddressMode");
        };
    }
}
