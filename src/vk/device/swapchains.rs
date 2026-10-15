use super::{Device, vk_image_usage};
use crate::vk::swapchain::SwapchainObject;
use crate::{CommandBuffer, Error, Swapchain, SwapchainInfo, Texture, TextureInfo, Timeline};
use ash::khr::swapchain;
use ash::vk;

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

    pub fn create_swapchain(
        &self,
        queue: u32,
        surface: vk::SurfaceKHR,
        info: &SwapchainInfo,
    ) -> Result<Swapchain, Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        let loader = self.loader();
        let mut create = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(info.images)
            .image_format(info.format.vk())
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(vk::Extent2D {
                width: info.width,
                height: info.height,
            })
            .image_array_layers(1)
            .image_usage(vk_image_usage(info.usage))
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(if info.vsync {
                vk::PresentModeKHR::FIFO
            } else {
                vk::PresentModeKHR::IMMEDIATE
            })
            .clipped(true);
        if self.families.len() > 1 {
            create = create
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families);
        }
        let raw = unsafe { loader.create_swapchain(&create, None) }?;
        let images = unsafe { loader.get_swapchain_images(raw) }
            .inspect_err(|_| unsafe { loader.destroy_swapchain(raw, None) })?;
        let texture = TextureInfo {
            format: info.format,
            width: info.width,
            height: info.height,
            usage: info.usage,
            ..Default::default()
        };
        let mut s = SwapchainObject {
            raw,
            queue,
            textures: Vec::new(),
            acquires: Vec::new(),
            presents: Vec::new(),
            state: Default::default(),
        };
        let made = images.iter().try_for_each(|&image| {
            s.textures.push(self.import_texture(image, &texture)?);
            let semaphore = vk::SemaphoreCreateInfo::default();
            s.acquires
                .push(unsafe { self.raw.create_semaphore(&semaphore, None) }?);
            s.presents
                .push(unsafe { self.raw.create_semaphore(&semaphore, None) }?);
            Ok(())
        });
        match made {
            Ok(()) => Ok(self.res.swapchains.insert(s)),
            Err(e) => {
                self.free_swapchain(s);
                Err(e)
            }
        }
    }

    pub fn destroy_swapchain(&self, swapchain: Swapchain) {
        let s = self.res.swapchains.remove(swapchain);
        self.free_swapchain(s);
    }

    pub(super) fn free_swapchain(&self, s: SwapchainObject) {
        for texture in s.textures {
            self.destroy_texture(texture);
        }
        for semaphore in s.acquires.into_iter().chain(s.presents) {
            unsafe { self.raw.destroy_semaphore(semaphore, None) };
        }
        unsafe { self.loader().destroy_swapchain(s.raw, None) };
    }

    pub fn acquire_texture(&self, swapchain: Swapchain) -> Result<Texture, Error> {
        let s = self.res.swapchains.get(swapchain);
        let mut state = s.state.lock().unwrap();
        let (next, current) = &mut *state;
        check!(
            current.is_none(),
            "acquire_texture on {swapchain} before submit_and_present presented the last one"
        );
        let semaphore = s.acquires[*next];
        let (image, _) = unsafe {
            self.loader()
                .acquire_next_image(s.raw, u64::MAX, semaphore, vk::Fence::null())
        }?;
        *current = Some((image, *next));
        *next = (*next + 1) % s.acquires.len();
        Ok(s.textures[image as usize])
    }

    pub fn submit_and_present<'d>(
        &self,
        swapchain: Swapchain,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), Error> {
        let s = self.res.swapchains.get(swapchain);
        let current = s.state.lock().unwrap().1.take();
        let Some((image, acquire)) = current else {
            panic!("submit_and_present on {swapchain} without acquire_texture");
        };
        self.submit_with(commands, waits, signals, Some((s, image, acquire)))
    }

    pub(super) fn present(
        &self,
        queue: vk::Queue,
        s: &SwapchainObject,
        image: u32,
    ) -> Result<(), Error> {
        let waits = [s.presents[image as usize]];
        let swapchains = [s.raw];
        let images = [image];
        let info = vk::PresentInfoKHR::default()
            .wait_semaphores(&waits)
            .swapchains(&swapchains)
            .image_indices(&images);
        unsafe { self.loader().queue_present(queue, &info) }?;
        Ok(())
    }
}
