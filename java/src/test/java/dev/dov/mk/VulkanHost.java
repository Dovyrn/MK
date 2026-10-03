package dev.dov.mk;

import static org.lwjgl.vulkan.EXTMutableDescriptorType.*;
import static org.lwjgl.vulkan.KHRPortabilityEnumeration.*;
import static org.lwjgl.vulkan.KHRPushDescriptor.*;
import static org.lwjgl.vulkan.VK13.*;

import lombok.experimental.UtilityClass;
import org.lwjgl.system.Configuration;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.vulkan.VK;
import org.lwjgl.vulkan.VkApplicationInfo;
import org.lwjgl.vulkan.VkDeviceCreateInfo;
import org.lwjgl.vulkan.VkDeviceQueueCreateInfo;
import org.lwjgl.vulkan.VkExtensionProperties;
import org.lwjgl.vulkan.VkInstance;
import org.lwjgl.vulkan.VkInstanceCreateInfo;
import org.lwjgl.vulkan.VkPhysicalDevice;
import org.lwjgl.vulkan.VkPhysicalDeviceFeatures2;
import org.lwjgl.vulkan.VkPhysicalDeviceMutableDescriptorTypeFeaturesEXT;
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

    public Device device() {
        Configuration.VULKAN_LIBRARY_NAME.set("libvulkan.1.dylib");
        try (var stack = MemoryStack.stackPush()) {
            var app = VkApplicationInfo.calloc(stack).sType$Default().apiVersion(VK_API_VERSION_1_3);
            var info = VkInstanceCreateInfo.calloc(stack)
                    .sType$Default()
                    .flags(VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR)
                    .pApplicationInfo(app)
                    .ppEnabledExtensionNames(stack.pointers(stack.UTF8(VK_KHR_PORTABILITY_ENUMERATION_EXTENSION_NAME)));
            var p = stack.mallocPointer(1);
            check(vkCreateInstance(info, null, p));
            var instance = new VkInstance(p.get(0), info);
            var count = stack.mallocInt(1);
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
            var subset = false;
            for (var e : available) {
                subset |= e.extensionNameString().equals("VK_KHR_portability_subset");
            }
            var extensions = stack.mallocPointer(subset ? 3 : 2);
            extensions.put(stack.UTF8(VK_EXT_MUTABLE_DESCRIPTOR_TYPE_EXTENSION_NAME));
            extensions.put(stack.UTF8(VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME));
            if (subset) {
                extensions.put(stack.UTF8("VK_KHR_portability_subset"));
            }
            extensions.flip();
            var mutable = VkPhysicalDeviceMutableDescriptorTypeFeaturesEXT.calloc(stack)
                    .sType$Default()
                    .mutableDescriptorType(true);
            var v13 = VkPhysicalDeviceVulkan13Features.calloc(stack)
                    .sType$Default()
                    .pNext(mutable.address())
                    .synchronization2(true)
                    .dynamicRendering(true)
                    .maintenance4(true);
            var v12 = VkPhysicalDeviceVulkan12Features.calloc(stack)
                    .sType$Default()
                    .pNext(v13.address())
                    .bufferDeviceAddress(true)
                    .timelineSemaphore(true)
                    .descriptorIndexing(true)
                    .runtimeDescriptorArray(true)
                    .descriptorBindingPartiallyBound(true)
                    .descriptorBindingSampledImageUpdateAfterBind(true)
                    .descriptorBindingStorageImageUpdateAfterBind(true)
                    .shaderSampledImageArrayNonUniformIndexing(true)
                    .shaderStorageImageArrayNonUniformIndexing(true);
            var v11 = VkPhysicalDeviceVulkan11Features.calloc(stack)
                    .sType$Default()
                    .pNext(v12.address())
                    .shaderDrawParameters(true);
            var features = VkPhysicalDeviceFeatures2.calloc(stack).sType$Default().pNext(v11.address());
            var queues = VkDeviceQueueCreateInfo.calloc(1, stack);
            queues.get(0).sType$Default().queueFamilyIndex(family).pQueuePriorities(stack.floats(1));
            var create = VkDeviceCreateInfo.calloc(stack)
                    .sType$Default()
                    .pNext(features.address())
                    .pQueueCreateInfos(queues)
                    .ppEnabledExtensionNames(extensions);
            check(vkCreateDevice(physical, create, null, p));
            var proc = VK.getFunctionProvider().getFunctionAddress("vkGetInstanceProcAddr");
            return VkDevice.adoptRaw(
                    proc, instance.address(), physical.address(), p.get(0), new int[] {family, 0}, features.address());
        }
    }
}
