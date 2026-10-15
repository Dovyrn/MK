use super::{Device, vk_image_usage};
use crate::vk::swapchain::{Present, State, SwapchainObject};
use crate::{
    ColorSpace, CommandBuffer, Error, PresentMode, SurfaceCaps, Swapchain, SwapchainInfo, Texture,
    TextureInfo, TextureUsage, Timeline,
};
use ash::khr::swapchain;
use ash::vk;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

fn vk_space(space: ColorSpace) -> vk::ColorSpaceKHR {
    match space {
        ColorSpace::Srgb => vk::ColorSpaceKHR::SRGB_NONLINEAR,
        ColorSpace::DisplayP3 => vk::ColorSpaceKHR::DISPLAY_P3_NONLINEAR_EXT,
        ColorSpace::ExtendedSrgb => vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
        ColorSpace::Hdr10 => vk::ColorSpaceKHR::HDR10_ST2084_EXT,
    }
}

fn vk_mode(mode: PresentMode) -> vk::PresentModeKHR {
    match mode {
        PresentMode::Fifo => vk::PresentModeKHR::FIFO,
        PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
    }
}

impl Device {
    pub(super) fn loader(&self) -> &swapchain::Device {
        self.ext
            .swapchain
            .as_ref()
            .expect("swapchains need DeviceInfo.swapchain")
    }

    pub fn raw_instance(&self) -> vk::Instance {
        self.instance.handle()
    }

    pub fn surface_caps(&self, surface: vk::SurfaceKHR) -> Result<SurfaceCaps, Error> {
        let caps = unsafe {
            self.surface
                .get_physical_device_surface_capabilities(self.physical, surface)
        }?;
        let modes = unsafe {
            self.surface
                .get_physical_device_surface_present_modes(self.physical, surface)
        }?;
        let fixed = caps.current_extent.width != u32::MAX;
        Ok(SurfaceCaps {
            min_images: caps.min_image_count,
            max_images: match caps.max_image_count {
                0 => u32::MAX,
                n => n,
            },
            width: if fixed { caps.current_extent.width } else { 0 },
            height: if fixed { caps.current_extent.height } else { 0 },
            mailbox: modes.contains(&vk::PresentModeKHR::MAILBOX),
            immediate: modes.contains(&vk::PresentModeKHR::IMMEDIATE),
        })
    }

    pub fn surface_format(
        &self,
        surface: vk::SurfaceKHR,
        format: crate::Format,
        space: ColorSpace,
    ) -> Result<bool, Error> {
        let formats = unsafe {
            self.surface
                .get_physical_device_surface_formats(self.physical, surface)
        }?;
        Ok(formats
            .iter()
            .any(|f| f.format == format.vk() && f.color_space == vk_space(space)))
    }

    fn check_surface(
        &self,
        queue: usize,
        surface: vk::SurfaceKHR,
        info: &SwapchainInfo,
    ) -> Result<(), Error> {
        let family = self.queues[queue].family;
        let presents = unsafe {
            self.surface
                .get_physical_device_surface_support(self.physical, family, surface)
        }?;
        check!(presents, "queue {queue} cannot present to this surface");
        let format = info.format;
        let space = info.color_space;
        check!(
            self.surface_format(surface, format, space)?,
            "the surface does not take {format} in {space:?}"
        );
        let caps = unsafe {
            self.surface
                .get_physical_device_surface_capabilities(self.physical, surface)
        }?;
        let images = info.images;
        let (min, max) = (caps.min_image_count, caps.max_image_count);
        check!(
            images >= min && (max == 0 || images <= max),
            "swapchain images {images} is outside the surface's {min}..={max}"
        );
        let (low, high) = (caps.min_image_extent, caps.max_image_extent);
        check!(
            (low.width..=high.width).contains(&info.width)
                && (low.height..=high.height).contains(&info.height),
            "swapchain size {}x{} is outside the surface's {}x{}..={}x{}",
            info.width,
            info.height,
            low.width,
            low.height,
            high.width,
            high.height
        );
        let usage = vk_image_usage(info.usage);
        check!(
            caps.supported_usage_flags.contains(usage),
            "the surface does not take swapchain usage {:#x}",
            info.usage
        );
        check!(
            caps.supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY),
            "the surface has no identity transform"
        );
        check!(
            caps.supported_composite_alpha
                .contains(vk::CompositeAlphaFlagsKHR::OPAQUE),
            "the surface has no opaque composite alpha"
        );
        let modes = unsafe {
            self.surface
                .get_physical_device_surface_present_modes(self.physical, surface)
        }?;
        check!(
            modes.contains(&vk_mode(info.mode)),
            "the surface has no {:?} present mode",
            info.mode
        );
        Ok(())
    }

    fn build(
        &self,
        queue: usize,
        surface: vk::SurfaceKHR,
        info: &SwapchainInfo,
        old: vk::SwapchainKHR,
    ) -> Result<State, Error> {
        if cfg!(feature = "validation") {
            self.check_surface(queue, surface, info)?;
        }
        let loader = self.loader();
        let mut create = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(info.images)
            .image_format(info.format.vk())
            .image_color_space(vk_space(info.color_space))
            .image_extent(vk::Extent2D {
                width: info.width,
                height: info.height,
            })
            .image_array_layers(1)
            .image_usage(vk_image_usage(info.usage))
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk_mode(info.mode))
            .clipped(true)
            .old_swapchain(old);
        if self.families.len() > 1 {
            create = create
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families);
        }
        let mut state = State {
            raw: unsafe { loader.create_swapchain(&create, None) }?,
            ..Default::default()
        };
        let texture = TextureInfo {
            format: info.format,
            width: info.width,
            height: info.height,
            usage: info.usage & !TextureUsage::VIEW,
            ..Default::default()
        };
        let semaphore = || unsafe {
            self.raw
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
        };
        let mut fill = || -> Result<(), Error> {
            let images = unsafe { loader.get_swapchain_images(state.raw) }?;
            state.spare = semaphore()?;
            for image in images {
                state.owned.push(semaphore()?);
                state.presents.push(semaphore()?);
                let texture = self.import_texture(image, &texture)?;
                self.res.table.presentable.set(texture, Default::default());
                state.textures.push(texture);
            }
            Ok(())
        };
        match fill() {
            Ok(()) => Ok(state),
            Err(e) => {
                self.free_state(state);
                Err(e)
            }
        }
    }

    pub(super) fn free_state(&self, state: State) {
        for &texture in &state.textures {
            self.destroy_texture(texture);
        }
        for semaphore in state.semaphores() {
            unsafe { self.raw.destroy_semaphore(semaphore, None) };
        }
        unsafe { self.loader().destroy_swapchain(state.raw, None) };
    }

    fn check_idle(&self, name: &str, swapchain: Swapchain, queue: usize, state: &State) {
        let idles = self.queues[queue].idles.load(Ordering::Relaxed);
        check!(
            state.mark.is_none_or(|m| idles > m),
            "{name} on {swapchain} while its last present may be pending; call wait_idle({queue}) first"
        );
        check!(
            state.current.is_none(),
            "{name} on {swapchain} between acquire_texture and submit_and_present"
        );
    }

    pub fn create_swapchain(
        &self,
        queue: u32,
        surface: vk::SurfaceKHR,
        info: &SwapchainInfo,
    ) -> Result<Swapchain, Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        let state = self.build(queue, surface, info, vk::SwapchainKHR::null())?;
        Ok(self.res.swapchains.insert(SwapchainObject {
            queue,
            surface,
            state: Mutex::new(state),
        }))
    }

    pub fn recreate_swapchain(
        &self,
        swapchain: Swapchain,
        info: &SwapchainInfo,
    ) -> Result<(), Error> {
        let s = self.res.swapchains.get(swapchain);
        let mut state = s.state.lock().unwrap();
        self.check_idle("recreate_swapchain", swapchain, s.queue, &state);
        let fresh = self.build(s.queue, s.surface, info, state.raw)?;
        let old = std::mem::replace(&mut *state, fresh);
        state.count = old.count;
        self.free_state(old);
        Ok(())
    }

    pub fn destroy_swapchain(&self, swapchain: Swapchain) {
        let s = self.res.swapchains.remove(swapchain);
        let state = s.state.into_inner().unwrap();
        self.check_idle("destroy_swapchain", swapchain, s.queue, &state);
        self.free_state(state);
    }

    pub fn acquire_texture(&self, swapchain: Swapchain) -> Result<Texture, Error> {
        let s = self.res.swapchains.get(swapchain);
        let mut state = s.state.lock().unwrap();
        check!(
            state.current.is_none(),
            "acquire_texture on {swapchain} before submit_and_present presented the last one"
        );
        let semaphore = state.spare;
        let (image, suboptimal) = unsafe {
            self.loader()
                .acquire_next_image(state.raw, u64::MAX, semaphore, vk::Fence::null())
        }?;
        state.spare = std::mem::replace(&mut state.owned[image as usize], semaphore);
        state.current = Some((image, semaphore));
        state.suboptimal |= suboptimal;
        let texture = state.textures[image as usize];
        self.res.table.acquire(texture, true);
        Ok(texture)
    }

    pub fn submit_and_present<'d>(
        &self,
        swapchain: Swapchain,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<bool, Error> {
        let s = self.res.swapchains.get(swapchain);
        let mut state = s.state.lock().unwrap();
        let Some((image, acquire)) = state.current.take() else {
            panic!("submit_and_present on {swapchain} without acquire_texture");
        };
        state.count += 1;
        state.mark = Some(self.queues[s.queue].idles.load(Ordering::Relaxed));
        let suboptimal = std::mem::take(&mut state.suboptimal);
        let present = Present {
            raw: state.raw,
            image,
            acquire,
            signal: state.presents[image as usize],
            id: state.count,
        };
        self.res
            .table
            .acquire(state.textures[image as usize], false);
        drop(state);
        let late = self.submit_with(commands, waits, signals, Some((s.queue, present)))?;
        Ok(suboptimal || late)
    }

    pub(super) fn present(&self, queue: vk::Queue, p: &Present) -> Result<bool, Error> {
        let waits = [p.signal];
        let swapchains = [p.raw];
        let images = [p.image];
        let ids = [p.id];
        let mut id = vk::PresentIdKHR::default().present_ids(&ids);
        let mut info = vk::PresentInfoKHR::default()
            .wait_semaphores(&waits)
            .swapchains(&swapchains)
            .image_indices(&images);
        if self.ext.present_id {
            info = info.push_next(&mut id);
        }
        Ok(unsafe { self.loader().queue_present(queue, &info) }?)
    }

    pub fn wait_presented(
        &self,
        swapchain: Swapchain,
        present: u64,
        ms: u64,
    ) -> Result<bool, Error> {
        let wait = self
            .ext
            .present_wait
            .as_ref()
            .expect("wait_presented needs DeviceInfo.present_wait");
        let raw = self.res.swapchains.get(swapchain).state.lock().unwrap().raw;
        let timeout = ms.saturating_mul(1_000_000);
        match unsafe { wait.wait_for_present(raw, present, timeout) } {
            Ok(()) => Ok(true),
            Err(vk::Result::TIMEOUT) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}
