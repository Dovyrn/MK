package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class MeshPipelineStates {
    public CullMode cullMode = CullMode.NONE;
    public FrontFace frontFace = FrontFace.COUNTER_CLOCKWISE;
    public DepthStencilStateInfo depthStencil = new DepthStencilStateInfo();
    public int frontStencilReference = 0;
    public int backStencilReference = 0;
    public float depthBiasConstant = 0.0f;
    public float depthBiasSlope = 0.0f;
    public float depthBiasClamp = 0.0f;
    public float[] blendColor = new float[] {0.0f, 0.0f, 0.0f, 0.0f};
    public int dynamicStates = 0;

    public static final int SIZE = 48 + DepthStencilStateInfo.SIZE;

    int encodedSize() {
        return 48 + depthStencil.encodedSize();
    }

    long encode(long p) {
        memPutInt(p, cullMode.value);
        p += 4;
        memPutInt(p, frontFace.value);
        p += 4;
        p = depthStencil.encode(p);
        memPutInt(p, frontStencilReference);
        p += 4;
        memPutInt(p, backStencilReference);
        p += 4;
        memPutFloat(p, depthBiasConstant);
        p += 4;
        memPutFloat(p, depthBiasSlope);
        p += 4;
        memPutFloat(p, depthBiasClamp);
        p += 4;
        if (blendColor.length != 4) {
            throw new IllegalArgumentException("blendColor has 4 values");
        }
        for (var x : blendColor) {
            memPutFloat(p, x);
            p += 4;
        }
        memPutInt(p, dynamicStates);
        p += 4;
        return p;
    }

    static MeshPipelineStates decode(long p) {
        var r = new MeshPipelineStates();
        r.cullMode = CullMode.of(memGetInt(p));
        p += 4;
        r.frontFace = FrontFace.of(memGetInt(p));
        p += 4;
        r.depthStencil = DepthStencilStateInfo.decode(p);
        p += r.depthStencil.encodedSize();
        r.frontStencilReference = memGetInt(p);
        p += 4;
        r.backStencilReference = memGetInt(p);
        p += 4;
        r.depthBiasConstant = memGetFloat(p);
        p += 4;
        r.depthBiasSlope = memGetFloat(p);
        p += 4;
        r.depthBiasClamp = memGetFloat(p);
        p += 4;
        for (var i = 0; i < 4; i++) {
            r.blendColor[i] = memGetFloat(p);
            p += 4;
        }
        r.dynamicStates = memGetInt(p);
        p += 4;
        return r;
    }
}
