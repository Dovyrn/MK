package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class RenderPipelineInfo {
    public long bytecode = 0;
    public long bytecodeSize = 0;
    public String vertexEntrypoint = "";
    public String fragmentEntrypoint = "";
    public ColorTarget[] colorTargets = new ColorTarget[0];
    public Format depthFormat = null;
    public Format stencilFormat = null;
    public int sampleCount = 1;
    public FillMode fillMode = FillMode.FILL;
    public boolean alphaToCoverage = false;
    public boolean depthClamp = false;
    public boolean orderedReads = false;
    public boolean indirectBuffers = false;
    public VertexBufferLayout[] vertexBuffers = new VertexBufferLayout[0];
    public VertexAttribute[] vertexAttributes = new VertexAttribute[0];
    public VertexPipelineStates states = new VertexPipelineStates();
    public DescriptorLayout[] descriptorSetLayout = new DescriptorLayout[0];
    public String label = "";

    int encodedSize() {
        var colorTargetsSize = 4;
        for (var x : colorTargets) {
            colorTargetsSize += x.encodedSize();
        }
        var vertexBuffersSize = 4;
        for (var x : vertexBuffers) {
            vertexBuffersSize += x.encodedSize();
        }
        var vertexAttributesSize = 4;
        for (var x : vertexAttributes) {
            vertexAttributesSize += x.encodedSize();
        }
        var descriptorSetLayoutSize = 4;
        for (var x : descriptorSetLayout) {
            descriptorSetLayoutSize += 8;
        }
        return 60 + vertexEntrypoint.getBytes(StandardCharsets.UTF_8).length + fragmentEntrypoint.getBytes(StandardCharsets.UTF_8).length + colorTargetsSize + (depthFormat == null ? 0 : 4) + (stencilFormat == null ? 0 : 4) + vertexBuffersSize + vertexAttributesSize + states.encodedSize() + descriptorSetLayoutSize + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, bytecode);
        memPutLong(p + 8, bytecodeSize);
        p += 16;
        var vertexEntrypointBytes = vertexEntrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, vertexEntrypointBytes.length);
        memByteBuffer(p + 4, vertexEntrypointBytes.length).put(vertexEntrypointBytes);
        p += 4 + vertexEntrypointBytes.length;
        var fragmentEntrypointBytes = fragmentEntrypoint.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, fragmentEntrypointBytes.length);
        memByteBuffer(p + 4, fragmentEntrypointBytes.length).put(fragmentEntrypointBytes);
        p += 4 + fragmentEntrypointBytes.length;
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
        memPutInt(p, vertexBuffers.length);
        p += 4;
        for (var x : vertexBuffers) {
            p = x.encode(p);
        }
        memPutInt(p, vertexAttributes.length);
        p += 4;
        for (var x : vertexAttributes) {
            p = x.encode(p);
        }
        p = states.encode(p);
        memPutInt(p, descriptorSetLayout.length);
        p += 4;
        for (var x : descriptorSetLayout) {
            memPutLong(p, DescriptorLayout.raw(x));
            p += 8;
        }
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
