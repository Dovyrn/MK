package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TensorInfo {
    public int[] dimensions = new int[0];
    public int[] strides = new int[0];
    public int element = Element.F32;
    public int usage = Tensor.COMPUTE;
    public int memory = Memory.DEVICE;

    int encodedSize() {
        var dimensionsSize = 4;
        for (var x : dimensions) {
            dimensionsSize += 4;
        }
        var stridesSize = 4;
        for (var x : strides) {
            stridesSize += 4;
        }
        return 12 + dimensionsSize + stridesSize;
    }

    long encode(long p) {
        memPutInt(p, dimensions.length);
        p += 4;
        for (var x : dimensions) {
            memPutInt(p, x);
            p += 4;
        }
        memPutInt(p, strides.length);
        p += 4;
        for (var x : strides) {
            memPutInt(p, x);
            p += 4;
        }
        memPutInt(p, element);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        memPutInt(p, memory);
        p += 4;
        return p;
    }
}
