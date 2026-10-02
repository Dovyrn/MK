package dev.dov.mk;

import static org.lwjgl.glfw.GLFW.*;
import static org.lwjgl.system.MemoryUtil.NULL;

import lombok.experimental.UtilityClass;

@UtilityClass
public class GlHost {
    public Device device() {
        if (!glfwInit()) {
            throw new IllegalStateException("glfwInit failed");
        }
        glfwDefaultWindowHints();
        glfwWindowHint(GLFW_VISIBLE, GLFW_FALSE);
        glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 4);
        glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 1);
        glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
        glfwWindowHint(GLFW_OPENGL_FORWARD_COMPAT, GLFW_TRUE);
        var window = glfwCreateWindow(16, 16, "mk", NULL, NULL);
        if (window == NULL) {
            throw new IllegalStateException("no GL window");
        }
        glfwMakeContextCurrent(window);
        return GlDevice.adoptRaw(Functions.GetProcAddress);
    }
}
