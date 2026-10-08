package dev.dov.mk;

import lombok.RequiredArgsConstructor;

@RequiredArgsConstructor
public enum Format {
    NONE(0),
    A8_UNORM(1),
    R8_UNORM(2),
    R8_SRGB(3),
    R8_SNORM(4),
    R8_UINT(5),
    R8_SINT(6),
    RG8_UNORM(7),
    RG8_SRGB(8),
    RG8_SNORM(9),
    RG8_UINT(10),
    RG8_SINT(11),
    RGBA8_UNORM(12),
    RGBA8_SRGB(13),
    RGBA8_SNORM(14),
    RGBA8_UINT(15),
    RGBA8_SINT(16),
    BGRA8_UNORM(17),
    BGRA8_SRGB(18),
    B5G6R5_UNORM(19),
    A1BGR5_UNORM(20),
    ABGR4_UNORM(21),
    BGR5A1_UNORM(22),
    RGB10A2_UNORM(23),
    RGB10A2_UINT(24),
    BGR10A2_UNORM(25),
    RG11B10_FLOAT(26),
    RGB9E5_FLOAT(27),
    R16_UNORM(28),
    R16_SNORM(29),
    R16_UINT(30),
    R16_SINT(31),
    R16_FLOAT(32),
    RG16_UNORM(33),
    RG16_SNORM(34),
    RG16_UINT(35),
    RG16_SINT(36),
    RG16_FLOAT(37),
    RGBA16_UNORM(38),
    RGBA16_SNORM(39),
    RGBA16_UINT(40),
    RGBA16_SINT(41),
    RGBA16_FLOAT(42),
    R32_UINT(43),
    R32_SINT(44),
    R32_FLOAT(45),
    RG32_UINT(46),
    RG32_SINT(47),
    RG32_FLOAT(48),
    RGBA32_UINT(49),
    RGBA32_SINT(50),
    RGBA32_FLOAT(51),
    RGB32_UINT(52),
    RGB32_SINT(53),
    RGB32_FLOAT(54),
    R64_UINT(55),
    R64_SINT(56),
    R64_FLOAT(57),
    RG64_UINT(58),
    RG64_SINT(59),
    RG64_FLOAT(60),
    RGBA64_UINT(61),
    RGBA64_SINT(62),
    RGBA64_FLOAT(63),
    D16_UNORM(64),
    D24_UNORM_S8(65),
    D32_FLOAT(66),
    D32_FLOAT_S8(67),
    S8_UINT(68),
    BC1_UNORM(69),
    BC1_SRGB(70),
    BC2_UNORM(71),
    BC2_SRGB(72),
    BC3_UNORM(73),
    BC3_SRGB(74),
    BC4_UNORM(75),
    BC4_SNORM(76),
    BC5_UNORM(77),
    BC5_SNORM(78),
    BC6H_UFLOAT(79),
    BC6H_FLOAT(80),
    BC7_UNORM(81),
    BC7_SRGB(82),
    ETC2_RGB8_UNORM(83),
    ETC2_RGB8_SRGB(84),
    ETC2_RGB8A1_UNORM(85),
    ETC2_RGB8A1_SRGB(86),
    ETC2_RGBA8_UNORM(87),
    ETC2_RGBA8_SRGB(88),
    EAC_R11_UNORM(89),
    EAC_R11_SNORM(90),
    EAC_RG11_UNORM(91),
    EAC_RG11_SNORM(92),
    ASTC_4X4_UNORM(93),
    ASTC_4X4_SRGB(94),
    ASTC_4X4_FLOAT(95),
    ASTC_5X4_UNORM(96),
    ASTC_5X4_SRGB(97),
    ASTC_5X4_FLOAT(98),
    ASTC_5X5_UNORM(99),
    ASTC_5X5_SRGB(100),
    ASTC_5X5_FLOAT(101),
    ASTC_6X5_UNORM(102),
    ASTC_6X5_SRGB(103),
    ASTC_6X5_FLOAT(104),
    ASTC_6X6_UNORM(105),
    ASTC_6X6_SRGB(106),
    ASTC_6X6_FLOAT(107),
    ASTC_8X5_UNORM(108),
    ASTC_8X5_SRGB(109),
    ASTC_8X5_FLOAT(110),
    ASTC_8X6_UNORM(111),
    ASTC_8X6_SRGB(112),
    ASTC_8X6_FLOAT(113),
    ASTC_8X8_UNORM(114),
    ASTC_8X8_SRGB(115),
    ASTC_8X8_FLOAT(116),
    ASTC_10X5_UNORM(117),
    ASTC_10X5_SRGB(118),
    ASTC_10X5_FLOAT(119),
    ASTC_10X6_UNORM(120),
    ASTC_10X6_SRGB(121),
    ASTC_10X6_FLOAT(122),
    ASTC_10X8_UNORM(123),
    ASTC_10X8_SRGB(124),
    ASTC_10X8_FLOAT(125),
    ASTC_10X10_UNORM(126),
    ASTC_10X10_SRGB(127),
    ASTC_10X10_FLOAT(128),
    ASTC_12X10_UNORM(129),
    ASTC_12X10_SRGB(130),
    ASTC_12X10_FLOAT(131),
    ASTC_12X12_UNORM(132),
    ASTC_12X12_SRGB(133),
    ASTC_12X12_FLOAT(134);

    final int value;

    static Format of(int v) {
        return switch (v) {
            case 0 -> NONE;
            case 1 -> A8_UNORM;
            case 2 -> R8_UNORM;
            case 3 -> R8_SRGB;
            case 4 -> R8_SNORM;
            case 5 -> R8_UINT;
            case 6 -> R8_SINT;
            case 7 -> RG8_UNORM;
            case 8 -> RG8_SRGB;
            case 9 -> RG8_SNORM;
            case 10 -> RG8_UINT;
            case 11 -> RG8_SINT;
            case 12 -> RGBA8_UNORM;
            case 13 -> RGBA8_SRGB;
            case 14 -> RGBA8_SNORM;
            case 15 -> RGBA8_UINT;
            case 16 -> RGBA8_SINT;
            case 17 -> BGRA8_UNORM;
            case 18 -> BGRA8_SRGB;
            case 19 -> B5G6R5_UNORM;
            case 20 -> A1BGR5_UNORM;
            case 21 -> ABGR4_UNORM;
            case 22 -> BGR5A1_UNORM;
            case 23 -> RGB10A2_UNORM;
            case 24 -> RGB10A2_UINT;
            case 25 -> BGR10A2_UNORM;
            case 26 -> RG11B10_FLOAT;
            case 27 -> RGB9E5_FLOAT;
            case 28 -> R16_UNORM;
            case 29 -> R16_SNORM;
            case 30 -> R16_UINT;
            case 31 -> R16_SINT;
            case 32 -> R16_FLOAT;
            case 33 -> RG16_UNORM;
            case 34 -> RG16_SNORM;
            case 35 -> RG16_UINT;
            case 36 -> RG16_SINT;
            case 37 -> RG16_FLOAT;
            case 38 -> RGBA16_UNORM;
            case 39 -> RGBA16_SNORM;
            case 40 -> RGBA16_UINT;
            case 41 -> RGBA16_SINT;
            case 42 -> RGBA16_FLOAT;
            case 43 -> R32_UINT;
            case 44 -> R32_SINT;
            case 45 -> R32_FLOAT;
            case 46 -> RG32_UINT;
            case 47 -> RG32_SINT;
            case 48 -> RG32_FLOAT;
            case 49 -> RGBA32_UINT;
            case 50 -> RGBA32_SINT;
            case 51 -> RGBA32_FLOAT;
            case 52 -> RGB32_UINT;
            case 53 -> RGB32_SINT;
            case 54 -> RGB32_FLOAT;
            case 55 -> R64_UINT;
            case 56 -> R64_SINT;
            case 57 -> R64_FLOAT;
            case 58 -> RG64_UINT;
            case 59 -> RG64_SINT;
            case 60 -> RG64_FLOAT;
            case 61 -> RGBA64_UINT;
            case 62 -> RGBA64_SINT;
            case 63 -> RGBA64_FLOAT;
            case 64 -> D16_UNORM;
            case 65 -> D24_UNORM_S8;
            case 66 -> D32_FLOAT;
            case 67 -> D32_FLOAT_S8;
            case 68 -> S8_UINT;
            case 69 -> BC1_UNORM;
            case 70 -> BC1_SRGB;
            case 71 -> BC2_UNORM;
            case 72 -> BC2_SRGB;
            case 73 -> BC3_UNORM;
            case 74 -> BC3_SRGB;
            case 75 -> BC4_UNORM;
            case 76 -> BC4_SNORM;
            case 77 -> BC5_UNORM;
            case 78 -> BC5_SNORM;
            case 79 -> BC6H_UFLOAT;
            case 80 -> BC6H_FLOAT;
            case 81 -> BC7_UNORM;
            case 82 -> BC7_SRGB;
            case 83 -> ETC2_RGB8_UNORM;
            case 84 -> ETC2_RGB8_SRGB;
            case 85 -> ETC2_RGB8A1_UNORM;
            case 86 -> ETC2_RGB8A1_SRGB;
            case 87 -> ETC2_RGBA8_UNORM;
            case 88 -> ETC2_RGBA8_SRGB;
            case 89 -> EAC_R11_UNORM;
            case 90 -> EAC_R11_SNORM;
            case 91 -> EAC_RG11_UNORM;
            case 92 -> EAC_RG11_SNORM;
            case 93 -> ASTC_4X4_UNORM;
            case 94 -> ASTC_4X4_SRGB;
            case 95 -> ASTC_4X4_FLOAT;
            case 96 -> ASTC_5X4_UNORM;
            case 97 -> ASTC_5X4_SRGB;
            case 98 -> ASTC_5X4_FLOAT;
            case 99 -> ASTC_5X5_UNORM;
            case 100 -> ASTC_5X5_SRGB;
            case 101 -> ASTC_5X5_FLOAT;
            case 102 -> ASTC_6X5_UNORM;
            case 103 -> ASTC_6X5_SRGB;
            case 104 -> ASTC_6X5_FLOAT;
            case 105 -> ASTC_6X6_UNORM;
            case 106 -> ASTC_6X6_SRGB;
            case 107 -> ASTC_6X6_FLOAT;
            case 108 -> ASTC_8X5_UNORM;
            case 109 -> ASTC_8X5_SRGB;
            case 110 -> ASTC_8X5_FLOAT;
            case 111 -> ASTC_8X6_UNORM;
            case 112 -> ASTC_8X6_SRGB;
            case 113 -> ASTC_8X6_FLOAT;
            case 114 -> ASTC_8X8_UNORM;
            case 115 -> ASTC_8X8_SRGB;
            case 116 -> ASTC_8X8_FLOAT;
            case 117 -> ASTC_10X5_UNORM;
            case 118 -> ASTC_10X5_SRGB;
            case 119 -> ASTC_10X5_FLOAT;
            case 120 -> ASTC_10X6_UNORM;
            case 121 -> ASTC_10X6_SRGB;
            case 122 -> ASTC_10X6_FLOAT;
            case 123 -> ASTC_10X8_UNORM;
            case 124 -> ASTC_10X8_SRGB;
            case 125 -> ASTC_10X8_FLOAT;
            case 126 -> ASTC_10X10_UNORM;
            case 127 -> ASTC_10X10_SRGB;
            case 128 -> ASTC_10X10_FLOAT;
            case 129 -> ASTC_12X10_UNORM;
            case 130 -> ASTC_12X10_SRGB;
            case 131 -> ASTC_12X10_FLOAT;
            case 132 -> ASTC_12X12_UNORM;
            case 133 -> ASTC_12X12_SRGB;
            case 134 -> ASTC_12X12_FLOAT;
            default -> throw new IllegalArgumentException(v + " is no Format");
        };
    }
}
