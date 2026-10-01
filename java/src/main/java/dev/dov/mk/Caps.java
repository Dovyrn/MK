package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class Caps {
    public boolean mesh = false;
    public int push = 0;
    public int queues = 0;
    public int samplers = 0;
    public int textures = 0;
    public float tick = 0.0f;
    public boolean timestamps = false;
    public boolean precise = false;
    public boolean ray = false;
    public boolean generated = false;
    public int handleSize = 0;
    public int handleAlignment = 0;
    public int tableAlignment = 0;
    public boolean sparse = false;
    public boolean shadingRate = false;
    public boolean hostMemory = false;
    public int hostAlignment = 0;
    public boolean sharedMemory = false;
    public int instance = 0;
    public boolean memoryless = false;
    public int views = 0;

    public int size() {
        return 84;
    }

    public long write(long p) {
        memPutInt(p, mesh ? 1 : 0);
        p += 4;
        memPutInt(p, push);
        p += 4;
        memPutInt(p, queues);
        p += 4;
        memPutInt(p, samplers);
        p += 4;
        memPutInt(p, textures);
        p += 4;
        memPutFloat(p, tick);
        p += 4;
        memPutInt(p, timestamps ? 1 : 0);
        p += 4;
        memPutInt(p, precise ? 1 : 0);
        p += 4;
        memPutInt(p, ray ? 1 : 0);
        p += 4;
        memPutInt(p, generated ? 1 : 0);
        p += 4;
        memPutInt(p, handleSize);
        p += 4;
        memPutInt(p, handleAlignment);
        p += 4;
        memPutInt(p, tableAlignment);
        p += 4;
        memPutInt(p, sparse ? 1 : 0);
        p += 4;
        memPutInt(p, shadingRate ? 1 : 0);
        p += 4;
        memPutInt(p, hostMemory ? 1 : 0);
        p += 4;
        memPutInt(p, hostAlignment);
        p += 4;
        memPutInt(p, sharedMemory ? 1 : 0);
        p += 4;
        memPutInt(p, instance);
        p += 4;
        memPutInt(p, memoryless ? 1 : 0);
        p += 4;
        memPutInt(p, views);
        p += 4;
        return p;
    }

    public static Caps read(long p) {
        var r = new Caps();
        r.mesh = memGetInt(p) != 0;
        p += 4;
        r.push = memGetInt(p);
        p += 4;
        r.queues = memGetInt(p);
        p += 4;
        r.samplers = memGetInt(p);
        p += 4;
        r.textures = memGetInt(p);
        p += 4;
        r.tick = memGetFloat(p);
        p += 4;
        r.timestamps = memGetInt(p) != 0;
        p += 4;
        r.precise = memGetInt(p) != 0;
        p += 4;
        r.ray = memGetInt(p) != 0;
        p += 4;
        r.generated = memGetInt(p) != 0;
        p += 4;
        r.handleSize = memGetInt(p);
        p += 4;
        r.handleAlignment = memGetInt(p);
        p += 4;
        r.tableAlignment = memGetInt(p);
        p += 4;
        r.sparse = memGetInt(p) != 0;
        p += 4;
        r.shadingRate = memGetInt(p) != 0;
        p += 4;
        r.hostMemory = memGetInt(p) != 0;
        p += 4;
        r.hostAlignment = memGetInt(p);
        p += 4;
        r.sharedMemory = memGetInt(p) != 0;
        p += 4;
        r.instance = memGetInt(p);
        p += 4;
        r.memoryless = memGetInt(p) != 0;
        p += 4;
        r.views = memGetInt(p);
        p += 4;
        return r;
    }
}
