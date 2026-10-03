package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

class Block implements Runnable {
    long address = nmemAlloc(4096);

    @Override
    public void run() {
        nmemFree(address);
        address = 0;
    }
}
