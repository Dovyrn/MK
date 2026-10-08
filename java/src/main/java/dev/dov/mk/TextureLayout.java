package dev.dov.mk;

public interface TextureLayout {
    int UNDEFINED = 0;
    int GENERAL = 1;
    int READ = 2;
    int COLOR = 3;
    int DEPTH = 4;
    int DEPTH_READ = 5;
    int COPY_SRC = 6;
    int COPY_DST = 7;
    int PRESENT = 8;
}
