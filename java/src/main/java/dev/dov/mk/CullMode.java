package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum CullMode {
    NONE(0),
    FRONT(1),
    BACK(2);

    final int value;

    static CullMode of(int v) {
        return switch (v) {
            case 0 -> NONE;
            case 1 -> FRONT;
            case 2 -> BACK;
            default -> throw new IllegalArgumentException(v + " is no CullMode");
        };
    }
}
