package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class MeshPipelineInfo {
    public long bytecode = 0;
    public long bytecodeSize = 0;
    public String objectEntrypoint = "";
    public String meshEntrypoint = "";
    public String fragmentEntrypoint = "";
    public int[] objectThreadgroupSize = new int[] {0, 0, 0};
    public int[] meshThreadgroupSize = new int[] {0, 0, 0};
    public ColorTarget[] colorTargets = new ColorTarget[0];
    public Format depthFormat = null;
    public Format stencilFormat = null;
    public int sampleCount = 1;
    public FillMode fillMode = FillMode.FILL;
    public boolean alphaToCoverage = false;
    public boolean depthClamp = false;
    public boolean orderedReads = false;
    public boolean indirectBuffers = false;
    public MeshPipelineStates states = new MeshPipelineStates();
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
        return 96 + objectEntrypoint.getBytes(StandardCharsets.UTF_8).length + meshEntrypoint.getBytes(StandardCharsets.UTF_8).length + fragmentEntrypoint.getBytes(StandardCharsets.UTF_8).length + colorTargetsSize + (depthFormat == null ? 0 : 4) + (stencilFormat == null ? 0 : 4) + states.encodedSize() + descriptorSetLayoutSize + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, bytecode);
        memPutLong(p + 8, bytecodeSize);
        p += 16;
        var objectEntrypointBytes = objectEntrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, objectEntrypointBytes.length);
        memByteBuffer(p + 4, objectEntrypointBytes.length).put(objectEntrypointBytes);
        p += 4 + objectEntrypointBytes.length;
        var meshEntrypointBytes = meshEntrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, meshEntrypointBytes.length);
        memByteBuffer(p + 4, meshEntrypointBytes.length).put(meshEntrypointBytes);
        p += 4 + meshEntrypointBytes.length;
        var fragmentEntrypointBytes = fragmentEntrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, fragmentEntrypointBytes.length);
        memByteBuffer(p + 4, fragmentEntrypointBytes.length).put(fragmentEntrypointBytes);
        p += 4 + fragmentEntrypointBytes.length;
        if (objectThreadgroupSize.length != 3) {
            throw new IllegalArgumentException("objectThreadgroupSize has 3 values");
        }
        for (var x : objectThreadgroupSize) {
            memPutInt(p, x);
            p += 4;
        }
        if (meshThreadgroupSize.length != 3) {
            throw new IllegalArgumentException("meshThreadgroupSize has 3 values");
        }
        for (var x : meshThreadgroupSize) {
            memPutInt(p, x);
            p += 4;
        }
        memPutInt(p, colorTargets.length);
        p += 4;
        for (var x : colorTargets) {
            p = x.encode(p);
        }
        memPutInt(p, depthFormat == null ? 0 : 1);
        p += 4;
        if (depthFormat != null) {
            memPutInt(p, depthFormat.value);
            p += 4;
        }
        memPutInt(p, stencilFormat == null ? 0 : 1);
        p += 4;
        if (stencilFormat != null) {
            memPutInt(p, stencilFormat.value);
            p += 4;
        }
        memPutInt(p, sampleCount);
        p += 4;
        memPutInt(p, fillMode.value);
        p += 4;
        memPutInt(p, alphaToCoverage ? 1 : 0);
        p += 4;
        memPutInt(p, depthClamp ? 1 : 0);
        p += 4;
        memPutInt(p, orderedReads ? 1 : 0);
        p += 4;
        memPutInt(p, indirectBuffers ? 1 : 0);
        p += 4;
        p = states.encode(p);
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
