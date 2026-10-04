package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

class Block implements Runnable {
    long address = nmemAlloc(4096);

    Block() {
        if (address == NULL) {
            throw new OutOfMemoryError("command stream of 4096 bytes");
        }
    }

    @Override
    public void run() {
        nmemFree(address);
        address = 0;
    }
}
