package dev.dov.mk;

import lombok.experimental.UtilityClass;
import org.lwjgl.system.JNI;
import org.lwjgl.system.Library;
import org.lwjgl.system.macosx.ObjCRuntime;

@UtilityClass
public class MetalHost {
    public Device device() {
        var metal = Library.loadNative(MetalHost.class, "org.lwjgl", "/System/Library/Frameworks/Metal.framework/Metal");
        var create = metal.getFunctionAddress("MTLCreateSystemDefaultDevice");
        var raw = JNI.invokeP(create);
        var device = MtlDevice.adoptRaw(raw, 1);
        var send = ObjCRuntime.getLibrary().getFunctionAddress("objc_msgSend");
        JNI.invokePPV(raw, ObjCRuntime.sel_getUid("release"), send);
        return device;
    }
}
