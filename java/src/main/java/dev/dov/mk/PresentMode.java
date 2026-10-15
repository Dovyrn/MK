package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum PresentMode {
    FIFO(0),
    MAILBOX(1),
    IMMEDIATE(2);

    final int value;

    static PresentMode of(int v) {
        return switch (v) {
            case 0 -> FIFO;
            case 1 -> MAILBOX;
            case 2 -> IMMEDIATE;
            default -> throw new IllegalArgumentException(v + " is no PresentMode");
        };
    }
}
