package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Access {
    public final int INDIRECT_READ = 1;
    public final int INDEX_READ = 2;
    public final int VERTEX_READ = 4;
    public final int UNIFORM_READ = 8;
    public final int INPUT_READ = 16;
    public final int SHADER_READ = 32;
    public final int SHADER_WRITE = 64;
    public final int COLOR_READ = 128;
    public final int COLOR_WRITE = 256;
    public final int DEPTH_READ = 512;
    public final int DEPTH_WRITE = 1024;
    public final int COPY_READ = 2048;
    public final int COPY_WRITE = 4096;
    public final int HOST_READ = 8192;
    public final int HOST_WRITE = 16384;
    public final int MEMORY_READ = 32768;
    public final int MEMORY_WRITE = 65536;
}
