package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class Device implements AutoCloseable {
    protected long handle;

    public Device(long handle) {
        this.handle = handle;
    }

    @Override
    public void close() {
        if (handle != 0) {
            nFree(handle);
            handle = 0;
        }
    }

    private static native void nFree(long device);

    public synchronized Caps caps() {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, Caps.SIZE);
            nCaps(handle, out);
            return Caps.decode(out);
        }
    }

    private static native void nCaps(long device, long out);

    public synchronized String name() {
        return nName(handle);
    }

    private static native String nName(long device);

    public synchronized int createBuffer(long size, int memory) {
        return nCreateBuffer(handle, size, memory);
    }

    private static native int nCreateBuffer(long device, long size, int memory);

    public synchronized void destroyBuffer(int buffer) {
        nDestroyBuffer(handle, buffer);
    }

    private static native void nDestroyBuffer(long device, int buffer);

    public synchronized long map(int buffer) {
        return nMap(handle, buffer);
    }

    private static native long nMap(long device, int buffer);

    public synchronized long bufferHandle(int buffer) {
        return nBufferHandle(handle, buffer);
    }

    private static native long nBufferHandle(long device, int buffer);

    public synchronized void unmap(int buffer) {
        nUnmap(handle, buffer);
    }

    private static native void nUnmap(long device, int buffer);

    public synchronized Budget memoryBudget() {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, Budget.SIZE);
            nMemoryBudget(handle, out);
            return Budget.decode(out);
        }
    }

    private static native void nMemoryBudget(long device, long out);

    public synchronized int createPipeline(ComputeInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreatePipeline(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreatePipeline(long device, long info, int infoLength);

    public synchronized void destroyPipeline(int pipeline) {
        nDestroyPipeline(handle, pipeline);
    }

    private static native void nDestroyPipeline(long device, int pipeline);

    public synchronized int createRenderPipeline(RenderInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateRenderPipeline(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateRenderPipeline(long device, long info, int infoLength);

    public synchronized int createMeshPipeline(MeshInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateMeshPipeline(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateMeshPipeline(long device, long info, int infoLength);

    public synchronized void destroyRenderPipeline(int pipeline) {
        nDestroyRenderPipeline(handle, pipeline);
    }

    private static native void nDestroyRenderPipeline(long device, int pipeline);

    public synchronized int createDepthState(DepthInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateDepthState(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateDepthState(long device, long info, int infoLength);

    public synchronized void destroyDepthState(int state) {
        nDestroyDepthState(handle, state);
    }

    private static native void nDestroyDepthState(long device, int state);

    public synchronized int createTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateTexture(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateTexture(long device, long info, int infoLength);

    public synchronized void writeTexture(HostCopy copy, long data, long dataSize) {
        try (var stack = MemoryStack.stackPush()) {
            var copyAddress = nmemAlloc(copy.encodedSize());
            copy.encode(copyAddress);
            try {
                nWriteTexture(handle, copyAddress, copy.encodedSize(), data, dataSize);
            } finally {
                nmemFree(copyAddress);
            }
        }
    }

    private static native void nWriteTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized void readTexture(HostCopy copy, long data, long dataSize) {
        try (var stack = MemoryStack.stackPush()) {
            var copyAddress = nmemAlloc(copy.encodedSize());
            copy.encode(copyAddress);
            try {
                nReadTexture(handle, copyAddress, copy.encodedSize(), data, dataSize);
            } finally {
                nmemFree(copyAddress);
            }
        }
    }

    private static native void nReadTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized int formatUsage(int format) {
        return nFormatUsage(handle, format);
    }

    private static native int nFormatUsage(long device, int format);

    public synchronized int formatSamples(int format, int usage) {
        return nFormatSamples(handle, format, usage);
    }

    private static native int nFormatSamples(long device, int format, int usage);

    public synchronized int createView(int texture, ViewInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateView(handle, texture, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateView(long device, int texture, long info, int infoLength);

    public synchronized void destroyTexture(int texture) {
        nDestroyTexture(handle, texture);
    }

    private static native void nDestroyTexture(long device, int texture);

    public synchronized int createSampler(SamplerInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateSampler(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateSampler(long device, long info, int infoLength);

    public synchronized void destroySampler(int sampler) {
        nDestroySampler(handle, sampler);
    }

    private static native void nDestroySampler(long device, int sampler);

    public synchronized int createQueryHeap(int kind, int count) {
        return nCreateQueryHeap(handle, kind, count);
    }

    private static native int nCreateQueryHeap(long device, int kind, int count);

    public synchronized void destroyQueryHeap(int heap) {
        nDestroyQueryHeap(handle, heap);
    }

    private static native void nDestroyQueryHeap(long device, int heap);

    public synchronized void recordCache() {
        nRecordCache(handle);
    }

    private static native void nRecordCache(long device);

    public synchronized void saveCache(String path) {
        try (var stack = MemoryStack.stackPush()) {
            var pathBytes = stack.UTF8(path, false);
            nSaveCache(handle, memAddress(pathBytes), pathBytes.remaining());
        }
    }

    private static native void nSaveCache(long device, long path, int pathLength);

    public synchronized void loadCache(String path) {
        try (var stack = MemoryStack.stackPush()) {
            var pathBytes = stack.UTF8(path, false);
            nLoadCache(handle, memAddress(pathBytes), pathBytes.remaining());
        }
    }

    private static native void nLoadCache(long device, long path, int pathLength);

    public synchronized int createEvent() {
        return nCreateEvent(handle);
    }

    private static native int nCreateEvent(long device);

    public synchronized void destroyEvent(int event) {
        nDestroyEvent(handle, event);
    }

    private static native void nDestroyEvent(long device, int event);

    public synchronized long eventValue(int event) {
        return nEventValue(handle, event);
    }

    private static native long nEventValue(long device, int event);

    public synchronized boolean await(int event, long value, long ms) {
        return nWait(handle, event, value, ms);
    }

    private static native boolean nWait(long device, int event, long value, long ms);

    public synchronized void submit(int queue, Commands commands, long[] waits, long[] signals) {
        try (var stack = MemoryStack.stackPush()) {
            if (waits.length % 2 != 0) {
                throw new IllegalArgumentException("waits holds event and value pairs");
            }
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            if (signals.length % 2 != 0) {
                throw new IllegalArgumentException("signals holds event and value pairs");
            }
            var signalsAddress = stack.nmalloc(8, signals.length * 8);
            for (var i = 0; i < signals.length / 2; i++) {
                memPutInt(signalsAddress + i * 16, (int) signals[i * 2]);
                memPutLong(signalsAddress + i * 16 + 8, signals[i * 2 + 1]);
            }
            nSubmit(handle, queue, commands.getAddress(), commands.getSize(), waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nSubmit(long device, int queue, long commands, long commandsLength, long waits, int waitsLength, long signals, int signalsLength);

    public synchronized int createHostBuffer(long pointer, long size) {
        return nCreateHostBuffer(handle, pointer, size);
    }

    private static native int nCreateHostBuffer(long device, long pointer, long size);
}
