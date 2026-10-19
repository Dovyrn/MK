package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class VertexStates {
    public Topology topology = Topology.TRIANGLE;
    public CullMode cull = CullMode.NONE;
    public FrontFace face = FrontFace.CCW;
    public DepthStateInfo depth = new DepthStateInfo();
    public int frontReference = 0;
    public int backReference = 0;
    public float biasConstant = 0.0f;
    public float biasSlope = 0.0f;
    public float biasClamp = 0.0f;
    public float[] blendColor = new float[] {0.0f, 0.0f, 0.0f, 0.0f};
    public int dynamic = 0;

    public static final int SIZE = 52 + DepthStateInfo.SIZE;

    int encodedSize() {
        return 52 + depth.encodedSize();
    }

    long encode(long p) {
        memPutInt(p, topology.value);
        p += 4;
        memPutInt(p, cull.value);
        p += 4;
        memPutInt(p, face.value);
        p += 4;
        p = depth.encode(p);
        memPutInt(p, frontReference);
        p += 4;
        memPutInt(p, backReference);
        p += 4;
        memPutFloat(p, biasConstant);
        p += 4;
        memPutFloat(p, biasSlope);
        p += 4;
        memPutFloat(p, biasClamp);
        p += 4;
        if (blendColor.length != 4) {
            throw new IllegalArgumentException("blendColor has 4 values");
        }
        for (var x : blendColor) {
            memPutFloat(p, x);
            p += 4;
        }
        memPutInt(p, dynamic);
        p += 4;
        return p;
    }

    static VertexStates decode(long p) {
        var r = new VertexStates();
        r.topology = Topology.of(memGetInt(p));
        p += 4;
        r.cull = CullMode.of(memGetInt(p));
        p += 4;
        r.face = FrontFace.of(memGetInt(p));
        p += 4;
        r.depth = DepthStateInfo.decode(p);
        p += r.depth.encodedSize();
        r.frontReference = memGetInt(p);
        p += 4;
        r.backReference = memGetInt(p);
        p += 4;
        r.biasConstant = memGetFloat(p);
        p += 4;
        r.biasSlope = memGetFloat(p);
        p += 4;
        r.biasClamp = memGetFloat(p);
        p += 4;
        for (var i = 0; i < 4; i++) {
            r.blendColor[i] = memGetFloat(p);
            p += 4;
        }
        r.dynamic = memGetInt(p);
        p += 4;
        return r;
    }
}
