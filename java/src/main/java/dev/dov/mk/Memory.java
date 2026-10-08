package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum Memory {
    DEVICE(0),
    UPLOAD(1),
    READBACK(2),
    TRANSIENT(3);

    final int value;

    static Memory of(int v) {
        return switch (v) {
            case 0 -> DEVICE;
            case 1 -> UPLOAD;
            case 2 -> READBACK;
            case 3 -> TRANSIENT;
            default -> throw new IllegalArgumentException(v + " is no Memory");
        };
    }
}
