package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TilePipelineInfo {
    public long bytecode = 0;
    public long bytecodeSize = 0;
    public String entrypoint = "";
    public ColorTarget[] colorTargets = new ColorTarget[0];
    public int sampleCount = 1;
    public DescriptorLayout[] descriptorSetLayout = new DescriptorLayout[0];
    public PipelineCache cache = null;
    public String label = "";

    int encodedSize() {
        var colorTargetsSize = 4;
        for (var x : colorTargets) {
            colorTargetsSize += x.encodedSize();
        }
        var descriptorSetLayoutSize = 4;
        for (var x : descriptorSetLayout) {
            descriptorSetLayoutSize += 8;
        }
        return 36 + entrypoint.getBytes(StandardCharsets.UTF_8).length + colorTargetsSize + descriptorSetLayoutSize + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, bytecode);
        memPutLong(p + 8, bytecodeSize);
        p += 16;
        var entrypointBytes = entrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, entrypointBytes.length);
        memByteBuffer(p + 4, entrypointBytes.length).put(entrypointBytes);
        p += 4 + entrypointBytes.length;
        memPutInt(p, colorTargets.length);
        p += 4;
        for (var x : colorTargets) {
            p = x.encode(p);
        }
        memPutInt(p, sampleCount);
        p += 4;
        memPutInt(p, descriptorSetLayout.length);
        p += 4;
        for (var x : descriptorSetLayout) {
            memPutLong(p, DescriptorLayout.raw(x));
            p += 8;
        }
        memPutLong(p, PipelineCache.raw(cache));
        p += 8;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
