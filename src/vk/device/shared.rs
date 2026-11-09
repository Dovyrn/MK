use super::{Device, image_create_info, whole_view};
use crate::table::lock;
use crate::vk::queue::timeline;
use crate::{Error, Memory, Texture, TextureInfo, Timeline};
use ash::khr::{external_memory_win32, external_semaphore_win32};
use ash::vk;
use std::sync::Arc;

const TEXTURE: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::D3D11_TEXTURE;

impl Device {
    pub fn device_luid(&self) -> u64 {
        let mut id = vk::PhysicalDeviceIDProperties::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut id);
        unsafe {
            self.instance
                .get_physical_device_properties2(self.physical, &mut properties)
        };
        if id.device_luid_valid == 0 {
            0
        } else {
            u64::from_le_bytes(id.device_luid)
        }
    }

    fn win32(
        &self,
        name: &str,
    ) -> Result<
        &(
            external_memory_win32::Device,
            external_semaphore_win32::Device,
        ),
        Error,
    > {
        self.ext
            .win32
            .as_ref()
            .ok_or_else(|| Error::Unsupported(format!("{name} needs DeviceInfo.shared_textures")))
    }

    pub fn import_shared_texture(&self, handle: u64, info: &TextureInfo) -> Result<Texture, Error> {
        let (memory, _) = self.win32("import_shared_texture")?;
        self.texture(info)?;
        let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(TEXTURE);
        let create = self
            .share_image(image_create_info(info))
            .push_next(&mut external);
        let mut format = vk::PhysicalDeviceExternalImageFormatInfo::default().handle_type(TEXTURE);
        let query = vk::PhysicalDeviceImageFormatInfo2::default()
            .format(create.format)
            .ty(create.image_type)
            .tiling(create.tiling)
            .usage(create.usage)
            .flags(create.flags)
            .push_next(&mut format);
        let mut support = vk::ExternalImageFormatProperties::default();
        let mut properties = vk::ImageFormatProperties2::default().push_next(&mut support);
        unsafe {
            self.instance.get_physical_device_image_format_properties2(
                self.physical,
                &query,
                &mut properties,
            )
        }
        .map_err(Error::from)?;
        let importable = support
            .external_memory_properties
            .external_memory_features
            .contains(vk::ExternalMemoryFeatureFlags::IMPORTABLE);
        if !importable {
            return Err(Error::Unsupported(format!(
                "format {} cannot import a D3D11 texture here",
                info.format
            )));
        }
        let image = unsafe { self.raw.create_image(&create, None) }.map_err(Error::from)?;
        let destroy = || unsafe { self.raw.destroy_image(image, None) };
        let needs = unsafe { self.raw.get_image_memory_requirements(image) };
        let raw = handle as vk::HANDLE;
        let mut kinds = vk::MemoryWin32HandlePropertiesKHR::default();
        unsafe { memory.get_memory_win32_handle_properties(TEXTURE, raw, &mut kinds) }
            .map_err(Error::from)
            .inspect_err(|_| destroy())?;
        let index = self
            .memory_type(
                needs.memory_type_bits & kinds.memory_type_bits,
                Memory::Device,
            )
            .inspect_err(|_| destroy())?;
        let mut import = vk::ImportMemoryWin32HandleInfoKHR::default()
            .handle_type(TEXTURE)
            .handle(raw);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .push_next(&mut import)
            .push_next(&mut dedicated);
        let bound = self
            .allocate_bound(index, alloc, |m| unsafe {
                self.raw.bind_image_memory(image, m, 0)
            })
            .inspect_err(|_| destroy())?;
        let id = self.create_views(image, Some(Arc::new(bound)), &whole_view(info))?;
        self.res
            .table
            .textures
            .set(id, TextureInfo { label: "", ..*info });
        self.res.table.shared.set(id, ());
        lock(&self.res.table.residency).exempt_texture(id);
        Ok(id)
    }

    pub fn import_shared_timeline(&self, handle: u64) -> Result<Timeline, Error> {
        self.caps.check_timelines("import_shared_timeline")?;
        let (_, semaphore) = self.win32("import_shared_timeline")?;
        let raw = timeline(&self.raw).map_err(Error::from)?;
        let info = vk::ImportSemaphoreWin32HandleInfoKHR::default()
            .semaphore(raw)
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::D3D12_FENCE)
            .handle(handle as vk::HANDLE);
        if let Err(e) = unsafe { semaphore.import_semaphore_win32_handle(&info) } {
            unsafe { self.raw.destroy_semaphore(raw, None) };
            return Err(Error::from(e));
        }
        Ok(self.add_timeline(raw, true))
    }
}
