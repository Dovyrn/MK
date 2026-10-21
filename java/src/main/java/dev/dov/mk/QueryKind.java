package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum QueryKind {
    TIMESTAMP(0),
    OCCLUSION_ANY(1),
    OCCLUSION_COUNT(4);

    final int value;

    static QueryKind of(int v) {
        return switch (v) {
            case 0 -> TIMESTAMP;
            case 1 -> OCCLUSION_ANY;
            case 4 -> OCCLUSION_COUNT;
            default -> throw new IllegalArgumentException(v + " is no QueryKind");
        };
    }
}
