package dev.dov.mk;

import static org.lwjgl.vulkan.EXTHeadlessSurface.*;
import static org.lwjgl.vulkan.KHRPortabilityEnumeration.*;
import static org.lwjgl.vulkan.KHRSurface.*;
import static org.lwjgl.vulkan.KHRPushDescriptor.*;
import static org.lwjgl.vulkan.VK13.*;

import lombok.experimental.UtilityClass;
import org.lwjgl.system.Configuration;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.Platform;
import org.lwjgl.vulkan.VK;
import org.lwjgl.vulkan.VkApplicationInfo;
import org.lwjgl.vulkan.VkDeviceCreateInfo;
import org.lwjgl.vulkan.VkDeviceQueueCreateInfo;
import org.lwjgl.vulkan.VkExtensionProperties;
import org.lwjgl.vulkan.VkHeadlessSurfaceCreateInfoEXT;
import org.lwjgl.vulkan.VkInstance;
import org.lwjgl.vulkan.VkInstanceCreateInfo;
import org.lwjgl.vulkan.VkPhysicalDevice;
import org.lwjgl.vulkan.VkPhysicalDeviceFeatures2;
import org.lwjgl.vulkan.VkPhysicalDeviceVulkan11Features;
import org.lwjgl.vulkan.VkPhysicalDeviceVulkan12Features;
import org.lwjgl.vulkan.VkPhysicalDeviceVulkan13Features;
import org.lwjgl.vulkan.VkQueueFamilyProperties;

@UtilityClass
public class VulkanHost {
    private void check(int result) {
        if (result != VK_SUCCESS) {
            throw new IllegalStateException("Vulkan error " + result);
        }
    }

    private boolean has(VkExtensionProperties.Buffer list, String name) {
        for (var e : list) {
            if (e.extensionNameString().equals(name)) {
                return true;
            }
        }
        return false;
    }

    public Device device() {
        if (Platform.get() == Platform.MACOSX) {
            Configuration.VULKAN_LIBRARY_NAME.set("libvulkan.1.dylib");
        }
        try (var stack = MemoryStack.stackPush()) {
            var app = VkApplicationInfo.calloc(stack).sType$Default().apiVersion(VK_API_VERSION_1_3);
            var count = stack.mallocInt(1);
            vkEnumerateInstanceExtensionProperties((String) null, count, null);
            var layers = VkExtensionProperties.malloc(count.get(0), stack);
            vkEnumerateInstanceExtensionProperties((String) null, count, layers);
            var portable = has(layers, VK_KHR_PORTABILITY_ENUMERATION_EXTENSION_NAME);
            var info = VkInstanceCreateInfo.calloc(stack).sType$Default().pApplicationInfo(app);
            if (portable) {
                info.flags(VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR)
                        .ppEnabledExtensionNames(stack.pointers(stack.UTF8(VK_KHR_PORTABILITY_ENUMERATION_EXTENSION_NAME)));
            }
            var p = stack.mallocPointer(1);
            check(vkCreateInstance(info, null, p));
            var instance = new VkInstance(p.get(0), info);
            check(vkEnumeratePhysicalDevices(instance, count, null));
            var physicals = stack.mallocPointer(count.get(0));
            check(vkEnumeratePhysicalDevices(instance, count, physicals));
            var physical = new VkPhysicalDevice(physicals.get(0), instance);
            vkGetPhysicalDeviceQueueFamilyProperties(physical, count, null);
            var families = VkQueueFamilyProperties.malloc(count.get(0), stack);
            vkGetPhysicalDeviceQueueFamilyProperties(physical, count, families);
            var family = 0;
            while ((families.get(family).queueFlags() & (VK_QUEUE_GRAPHICS_BIT | VK_QUEUE_COMPUTE_BIT)) == 0) {
                family++;
            }
            vkEnumerateDeviceExtensionProperties(physical, (String) null, count, null);
            var available = VkExtensionProperties.malloc(count.get(0), stack);
            vkEnumerateDeviceExtensionProperties(physical, (String) null, count, available);
            var subset = has(available, "VK_KHR_portability_subset");
            var push = has(available, VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME);
            var extensions = stack.mallocPointer(2);
            if (push) {
                extensions.put(stack.UTF8(VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME));
            }
            if (subset) {
                extensions.put(stack.UTF8("VK_KHR_portability_subset"));
            }
            extensions.flip();
            var v13 = VkPhysicalDeviceVulkan13Features.calloc(stack).sType$Default();
            var v12 = VkPhysicalDeviceVulkan12Features.calloc(stack).sType$Default().pNext(v13.address());
            var v11 = VkPhysicalDeviceVulkan11Features.calloc(stack).sType$Default().pNext(v12.address());
            var features = VkPhysicalDeviceFeatures2.calloc(stack).sType$Default().pNext(v11.address());
            vkGetPhysicalDeviceFeatures2(physical, features);
            var queues = VkDeviceQueueCreateInfo.calloc(1, stack);
            queues.get(0).sType$Default().queueFamilyIndex(family).pQueuePriorities(stack.floats(1));
            var create = VkDeviceCreateInfo.calloc(stack)
                    .sType$Default()
                    .pNext(features.address())
                    .pQueueCreateInfos(queues)
                    .ppEnabledExtensionNames(extensions);
            check(vkCreateDevice(physical, create, null, p));
            var proc = VK.getFunctionProvider().getFunctionAddress("vkGetInstanceProcAddr");
            return VkDevice.importRawDevice(
                    proc, instance.address(), physical.address(), p.get(0), new int[] {family, 0}, features.address());
        }
    }

    public boolean headless() {
        try (var stack = MemoryStack.stackPush()) {
            var count = stack.mallocInt(1);
            vkEnumerateInstanceExtensionProperties((String) null, count, null);
            var list = VkExtensionProperties.malloc(count.get(0), stack);
            vkEnumerateInstanceExtensionProperties((String) null, count, list);
            return has(list, VK_EXT_HEADLESS_SURFACE_EXTENSION_NAME);
        }
    }

    public VkInstance wrap(long raw) {
        try (var stack = MemoryStack.stackPush()) {
            var app = VkApplicationInfo.calloc(stack).sType$Default().apiVersion(VK_API_VERSION_1_3);
            var names = stack.pointers(stack.UTF8(VK_KHR_SURFACE_EXTENSION_NAME), stack.UTF8(VK_EXT_HEADLESS_SURFACE_EXTENSION_NAME));
            var info = VkInstanceCreateInfo.calloc(stack).sType$Default().pApplicationInfo(app).ppEnabledExtensionNames(names);
            return new VkInstance(raw, info);
        }
    }

    public long headless(VkInstance instance) {
        try (var stack = MemoryStack.stackPush()) {
            var surface = stack.mallocLong(1);
            check(vkCreateHeadlessSurfaceEXT(instance, VkHeadlessSurfaceCreateInfoEXT.calloc(stack).sType$Default(), null, surface));
            return surface.get(0);
        }
    }
}
