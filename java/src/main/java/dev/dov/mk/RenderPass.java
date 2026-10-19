package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class RenderPass {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    RenderPass(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording.incrementAndGet();
        }
    }

    static RenderPass adopt(Device device, long handle) {
        return new RenderPass(device, null, handle);
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("pass used after end");
        }
        return handle;
    }

    public void end() {
        var ended = handle();
        handle = 0;
        nEnd(ended);
        if (owner != null) {
            owner.ended();
        } else {
            device.recording.decrementAndGet();
        }
    }

    public long raw() {
        return nRaw(handle());
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);

    public void beginDebugLabel(String text) {
        try (var arena = new Arena()) {
            var encoded = text.getBytes(StandardCharsets.UTF_8);
            nBeginDebugLabel(handle(), arena.bytes(encoded), encoded.length);
        }
    }

    private static native void nBeginDebugLabel(long target, long text, int textLength);

    public void endDebugLabel() {
        nEndDebugLabel(handle());
    }

    private static native void nEndDebugLabel(long target);

    public void bindRenderPipeline(RenderPipeline pipeline) {
        nBindRenderPipeline(handle(), RenderPipeline.raw(pipeline));
    }

    private static native void nBindRenderPipeline(long target, long pipeline);

    public void setCullMode(CullMode cull) {
        nSetCullMode(handle(), cull.value);
    }

    private static native void nSetCullMode(long target, int cull);

    public void setFrontFace(FrontFace face) {
        nSetFrontFace(handle(), face.value);
    }

    private static native void nSetFrontFace(long target, int face);

    public void setPrimitiveTopology(Topology topology) {
        nSetPrimitiveTopology(handle(), topology.value);
    }

    private static native void nSetPrimitiveTopology(long target, int topology);

    public void setDepthState(DepthState state) {
        nSetDepthState(handle(), DepthState.raw(state));
    }

    private static native void nSetDepthState(long target, long state);

    public void setStencilReference(int front, int back) {
        nSetStencilReference(handle(), front, back);
    }

    private static native void nSetStencilReference(long target, int front, int back);

    public void setDepthBias(float constant, float slope, float clamp) {
        nSetDepthBias(handle(), constant, slope, clamp);
    }

    private static native void nSetDepthBias(long target, float constant, float slope, float clamp);

    public void setBlendColor(float[] color) {
        nSetBlendColor(handle(), color[0], color[1], color[2], color[3]);
    }

    private static native void nSetBlendColor(long target, float color0, float color1, float color2, float color3);

    public void bindDescriptorSet(DescriptorSet set) {
        nBindDescriptorSet(handle(), DescriptorSet.raw(set));
    }

    private static native void nBindDescriptorSet(long target, long set);

    public void bindDescriptorSetDynamic(DescriptorSet set, int[] offsets) {
        try (var arena = new Arena()) {
            var offsetsSize = 4;
            for (var x : offsets) {
                offsetsSize += 4;
            }
            var list = arena.alloc(offsetsSize);
            var at = list;
            memPutInt(at, offsets.length);
            at += 4;
            for (var x : offsets) {
                memPutInt(at, x);
                at += 4;
            }
            nBindDescriptorSetDynamic(handle(), DescriptorSet.raw(set), list, offsetsSize);
        }
    }

    private static native void nBindDescriptorSetDynamic(long target, long set, long offsets, int offsetsLength);

    public void bindVertexBuffer(int slot, Buffer buffer, long offset) {
        nBindVertexBuffer(handle(), slot, Buffer.raw(buffer), offset);
    }

    private static native void nBindVertexBuffer(long target, int slot, long buffer, long offset);

    public void bindIndexBuffer(Buffer buffer, long offset, IndexType kind) {
        nBindIndexBuffer(handle(), Buffer.raw(buffer), offset, kind.value);
    }

    private static native void nBindIndexBuffer(long target, long buffer, long offset, int kind);

    public void draw(int vertices, int instances, int firstVertex, int firstInstance) {
        nDraw(handle(), vertices, instances, firstVertex, firstInstance);
    }

    private static native void nDraw(long target, int vertices, int instances, int firstVertex, int firstInstance);

    public void drawIndexed(int indices, int instances, int firstIndex, int baseVertex, int firstInstance) {
        nDrawIndexed(handle(), indices, instances, firstIndex, baseVertex, firstInstance);
    }

    private static native void nDrawIndexed(long target, int indices, int instances, int firstIndex, int baseVertex, int firstInstance);

    public void drawIndirect(Buffer buffer, long offset) {
        nDrawIndirect(handle(), Buffer.raw(buffer), offset);
    }

    private static native void nDrawIndirect(long target, long buffer, long offset);

    public void drawIndexedIndirect(Buffer buffer, long offset) {
        nDrawIndexedIndirect(handle(), Buffer.raw(buffer), offset);
    }

    private static native void nDrawIndexedIndirect(long target, long buffer, long offset);

    public void executeIndirect(IndirectBuffer indirect, int first, int count) {
        nExecuteIndirect(handle(), IndirectBuffer.raw(indirect), first, count);
    }

    private static native void nExecuteIndirect(long target, long indirect, int first, int count);

    public void executeIndirectCount(IndirectBuffer indirect, Buffer buffer, long offset) {
        nExecuteIndirectCount(handle(), IndirectBuffer.raw(indirect), Buffer.raw(buffer), offset);
    }

    private static native void nExecuteIndirectCount(long target, long indirect, long buffer, long offset);

    public void drawMeshTasks(int x, int y, int z) {
        nDrawMeshTasks(handle(), x, y, z);
    }

    private static native void nDrawMeshTasks(long target, int x, int y, int z);

    public void drawMeshTasksIndirect(Buffer buffer, long offset) {
        nDrawMeshTasksIndirect(handle(), Buffer.raw(buffer), offset);
    }

    private static native void nDrawMeshTasksIndirect(long target, long buffer, long offset);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void beginOcclusionQuery(QueryPool pool, int index) {
        nBeginOcclusionQuery(handle(), QueryPool.raw(pool), index);
    }

    private static native void nBeginOcclusionQuery(long target, long pool, int index);

    public void endOcclusionQuery(QueryPool pool, int index) {
        nEndOcclusionQuery(handle(), QueryPool.raw(pool), index);
    }

    private static native void nEndOcclusionQuery(long target, long pool, int index);

    public void setViewport(Viewport[] viewports) {
        try (var arena = new Arena()) {
            var viewportsSize = 4;
            for (var x : viewports) {
                viewportsSize += x.encodedSize();
            }
            var list = arena.alloc(viewportsSize);
            var at = list;
            memPutInt(at, viewports.length);
            at += 4;
            for (var x : viewports) {
                at = x.encode(at);
            }
            nSetViewport(handle(), list, viewportsSize);
        }
    }

    private static native void nSetViewport(long target, long viewports, int viewportsLength);

    public void setScissor(Scissor[] scissors) {
        try (var arena = new Arena()) {
            var scissorsSize = 4;
            for (var x : scissors) {
                scissorsSize += x.encodedSize();
            }
            var list = arena.alloc(scissorsSize);
            var at = list;
            memPutInt(at, scissors.length);
            at += 4;
            for (var x : scissors) {
                at = x.encode(at);
            }
            nSetScissor(handle(), list, scissorsSize);
        }
    }

    private static native void nSetScissor(long target, long scissors, int scissorsLength);

    public void inputAttachmentBarrier() {
        nInputAttachmentBarrier(handle());
    }

    private static native void nInputAttachmentBarrier(long target);

    public void dispatchTile(TilePipeline pipeline) {
        nDispatchTile(handle(), TilePipeline.raw(pipeline));
    }

    private static native void nDispatchTile(long target, long pipeline);
}
