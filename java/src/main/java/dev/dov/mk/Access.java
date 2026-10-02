package dev.dov.mk;

public interface Access {
    int INDIRECT_READ = 1;
    int INDEX_READ = 2;
    int VERTEX_READ = 4;
    int UNIFORM_READ = 8;
    int INPUT_READ = 16;
    int SHADER_READ = 32;
    int SHADER_WRITE = 64;
    int COLOR_READ = 128;
    int COLOR_WRITE = 256;
    int DEPTH_READ = 512;
    int DEPTH_WRITE = 1024;
    int COPY_READ = 2048;
    int COPY_WRITE = 4096;
    int HOST_READ = 8192;
    int HOST_WRITE = 16384;
    int MEMORY_READ = 32768;
    int MEMORY_WRITE = 65536;
    int ACCEL_READ = 2097152;
    int ACCEL_WRITE = 4194304;
}
