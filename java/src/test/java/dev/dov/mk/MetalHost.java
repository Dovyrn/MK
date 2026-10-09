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
        var send = ObjCRuntime.getLibrary().getFunctionAddress("objc_msgSend");
        var descriptor = JNI.invokePPP(ObjCRuntime.objc_getClass("MTL4CompilerDescriptor"), ObjCRuntime.sel_getUid("new"), send);
        var compiler = JNI.invokePPPPP(raw, ObjCRuntime.sel_getUid("newCompilerWithDescriptor:error:"), descriptor, 0L, send);
        var device = MtlDevice.importDeviceRaw(raw, compiler, 1);
        for (var object : new long[] {compiler, descriptor, raw}) {
            JNI.invokePPV(object, ObjCRuntime.sel_getUid("release"), send);
        }
        return device;
    }
}
