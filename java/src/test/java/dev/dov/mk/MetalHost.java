package dev.dov.mk;

import lombok.experimental.UtilityClass;
import org.lwjgl.system.JNI;
import org.lwjgl.system.Library;

@UtilityClass
public class MetalHost {
    public Device device() {
        var metal = Library.loadNative(MetalHost.class, "org.lwjgl", "/System/Library/Frameworks/Metal.framework/Metal");
        var create = metal.getFunctionAddress("MTLCreateSystemDefaultDevice");
        return MtlDevice.adoptRaw(JNI.invokeP(create), 1);
    }
}
