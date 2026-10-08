package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum QueryKind {
    TIMESTAMP(0),
    OCCLUSION(1),
    PRECISE(4);

    final int value;

    static QueryKind of(int v) {
        return switch (v) {
            case 0 -> TIMESTAMP;
            case 1 -> OCCLUSION;
            case 4 -> PRECISE;
            default -> throw new IllegalArgumentException(v + " is no QueryKind");
        };
    }
}
