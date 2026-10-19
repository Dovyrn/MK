package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum DescriptorType {
    UNIFORM_BUFFER(0),
    STORAGE_BUFFER(1),
    TEXTURE(2),
    STORAGE_TEXTURE(3),
    INPUT_ATTACHMENT(4),
    SAMPLER(5),
    UNIFORM_BUFFER_DYNAMIC(6),
    STORAGE_BUFFER_DYNAMIC(7);

    final int value;

    static DescriptorType of(int v) {
        return switch (v) {
            case 0 -> UNIFORM_BUFFER;
            case 1 -> STORAGE_BUFFER;
            case 2 -> TEXTURE;
            case 3 -> STORAGE_TEXTURE;
            case 4 -> INPUT_ATTACHMENT;
            case 5 -> SAMPLER;
            case 6 -> UNIFORM_BUFFER_DYNAMIC;
            case 7 -> STORAGE_BUFFER_DYNAMIC;
            default -> throw new IllegalArgumentException(v + " is no DescriptorType");
        };
    }
}
