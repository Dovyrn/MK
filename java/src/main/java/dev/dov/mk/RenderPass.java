package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class RenderPass implements AutoCloseable {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    RenderPass(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording++;
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
        synchronized (device) {
            var ended = handle();
            handle = 0;
            if (owner != null) {
                owner.ended();
            } else {
                device.recording--;
            }
            nEnd(ended);
        }
    }

    public long raw() {
        synchronized (device) {
            return nRaw(handle());
        }
    }

    @Override
    public void close() {
        if (handle != 0) {
            end();
        }
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);

    public void beginDebugLabel(String text) {
        synchronized (device) {
            try (var arena = new Arena()) {
            var encoded = text.getBytes(StandardCharsets.UTF_8);
                nBeginDebugLabel(handle(), arena.bytes(encoded), encoded.length);
            }
        }
    }

    private static native void nBeginDebugLabel(long target, long text, int textLength);

    public void endDebugLabel() {
        synchronized (device) {
            nEndDebugLabel(handle());
        }
    }

    private static native void nEndDebugLabel(long target);

    public void bindRenderPipeline(RenderPipeline pipeline) {
        synchronized (device) {
            nBindRenderPipeline(handle(), RenderPipeline.raw(pipeline));
        }
    }

    private static native void nBindRenderPipeline(long target, long pipeline);

    public void setCullMode(int cull) {
        synchronized (device) {
            nSetCullMode(handle(), cull);
        }
    }

    private static native void nSetCullMode(long target, int cull);

    public void setFrontFace(int face) {
        synchronized (device) {
            nSetFrontFace(handle(), face);
        }
    }

    private static native void nSetFrontFace(long target, int face);

    public void setPrimitiveTopology(int topology) {
        synchronized (device) {
            nSetPrimitiveTopology(handle(), topology);
        }
    }

    private static native void nSetPrimitiveTopology(long target, int topology);

    public void setDepthState(DepthState state) {
        synchronized (device) {
            nSetDepthState(handle(), DepthState.raw(state));
        }
    }

    private static native void nSetDepthState(long target, long state);

    public void setStencilReference(int front, int back) {
        synchronized (device) {
            nSetStencilReference(handle(), front, back);
        }
    }

    private static native void nSetStencilReference(long target, int front, int back);

    public void setDepthBias(float constant, float slope, float clamp) {
        synchronized (device) {
            nSetDepthBias(handle(), constant, slope, clamp);
        }
    }

    private static native void nSetDepthBias(long target, float constant, float slope, float clamp);

    public void setBlendColor(float r, float g, float b, float a) {
        synchronized (device) {
            nSetBlendColor(handle(), r, g, b, a);
        }
    }

    private static native void nSetBlendColor(long target, float r, float g, float b, float a);

    public void bindUniformBuffer(int slot, Buffer buffer, long offset, long size) {
        synchronized (device) {
            nBindUniformBuffer(handle(), slot, Buffer.raw(buffer), offset, size);
        }
    }

    private static native void nBindUniformBuffer(long target, int slot, long buffer, long offset, long size);

    public void bindStorageBuffer(int slot, Buffer buffer, long offset, long size) {
        synchronized (device) {
            nBindStorageBuffer(handle(), slot, Buffer.raw(buffer), offset, size);
        }
    }

    private static native void nBindStorageBuffer(long target, int slot, long buffer, long offset, long size);

    public void bindTexture(int slot, Texture texture) {
        synchronized (device) {
            nBindTexture(handle(), slot, Texture.raw(texture));
        }
    }

    private static native void nBindTexture(long target, int slot, long texture);

    public void bindStorageTexture(int slot, Texture texture) {
        synchronized (device) {
            nBindStorageTexture(handle(), slot, Texture.raw(texture));
        }
    }

    private static native void nBindStorageTexture(long target, int slot, long texture);

    public void bindSampler(int slot, Sampler sampler) {
        synchronized (device) {
            nBindSampler(handle(), slot, Sampler.raw(sampler));
        }
    }

    private static native void nBindSampler(long target, int slot, long sampler);

    public void bindVertexBuffer(int slot, Buffer buffer, long offset) {
        synchronized (device) {
            nBindVertexBuffer(handle(), slot, Buffer.raw(buffer), offset);
        }
    }

    private static native void nBindVertexBuffer(long target, int slot, long buffer, long offset);

    public void bindIndexBuffer(Buffer buffer, long offset, int kind) {
        synchronized (device) {
            nBindIndexBuffer(handle(), Buffer.raw(buffer), offset, kind);
        }
    }

    private static native void nBindIndexBuffer(long target, long buffer, long offset, int kind);

    public void draw(int vertices, int instances, int firstVertex, int firstInstance) {
        synchronized (device) {
            nDraw(handle(), vertices, instances, firstVertex, firstInstance);
        }
    }

    private static native void nDraw(long target, int vertices, int instances, int firstVertex, int firstInstance);

    public void drawIndexed(int indices, int instances, int firstIndex, int baseVertex, int firstInstance) {
        synchronized (device) {
            nDrawIndexed(handle(), indices, instances, firstIndex, baseVertex, firstInstance);
        }
    }

    private static native void nDrawIndexed(long target, int indices, int instances, int firstIndex, int baseVertex, int firstInstance);

    public void drawIndirect(Buffer buffer, long offset) {
        synchronized (device) {
            nDrawIndirect(handle(), Buffer.raw(buffer), offset);
        }
    }

    private static native void nDrawIndirect(long target, long buffer, long offset);

    public void drawIndexedIndirect(Buffer buffer, long offset) {
        synchronized (device) {
            nDrawIndexedIndirect(handle(), Buffer.raw(buffer), offset);
        }
    }

    private static native void nDrawIndexedIndirect(long target, long buffer, long offset);

    public void executeIndirect(IndirectBuffer indirect, int first, int count) {
        synchronized (device) {
            nExecuteIndirect(handle(), IndirectBuffer.raw(indirect), first, count);
        }
    }

    private static native void nExecuteIndirect(long target, long indirect, int first, int count);

    public void executeIndirectCount(IndirectBuffer indirect, Buffer buffer, long offset) {
        synchronized (device) {
            nExecuteIndirectCount(handle(), IndirectBuffer.raw(indirect), Buffer.raw(buffer), offset);
        }
    }

    private static native void nExecuteIndirectCount(long target, long indirect, long buffer, long offset);

    public void drawMeshTasks(int x, int y, int z) {
        synchronized (device) {
            nDrawMeshTasks(handle(), x, y, z);
        }
    }

    private static native void nDrawMeshTasks(long target, int x, int y, int z);

    public void drawMeshTasksIndirect(Buffer buffer, long offset) {
        synchronized (device) {
            nDrawMeshTasksIndirect(handle(), Buffer.raw(buffer), offset);
        }
    }

    private static native void nDrawMeshTasksIndirect(long target, long buffer, long offset);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        synchronized (device) {
            nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
        }
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void beginOcclusionQuery(QueryPool pool, int index) {
        synchronized (device) {
            nBeginOcclusionQuery(handle(), QueryPool.raw(pool), index);
        }
    }

    private static native void nBeginOcclusionQuery(long target, long pool, int index);

    public void endOcclusionQuery(QueryPool pool, int index) {
        synchronized (device) {
            nEndOcclusionQuery(handle(), QueryPool.raw(pool), index);
        }
    }

    private static native void nEndOcclusionQuery(long target, long pool, int index);

    public void setViewport(Viewport[] viewports) {
        synchronized (device) {
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
    }

    private static native void nSetViewport(long target, long viewports, int viewportsLength);

    public void setScissor(Scissor[] scissors) {
        synchronized (device) {
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
    }

    private static native void nSetScissor(long target, long scissors, int scissorsLength);

    public void inputAttachmentBarrier() {
        synchronized (device) {
            nInputAttachmentBarrier(handle());
        }
    }

    private static native void nInputAttachmentBarrier(long target);

    public void dispatchTile(TilePipeline pipeline) {
        synchronized (device) {
            nDispatchTile(handle(), TilePipeline.raw(pipeline));
        }
    }

    private static native void nDispatchTile(long target, long pipeline);
}
