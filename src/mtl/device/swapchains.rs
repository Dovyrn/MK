use super::Device;
use crate::mtl::swapchain::SwapchainObject;
use crate::{CommandBuffer, Error, Swapchain, SwapchainInfo, Texture, TextureUsage, Timeline};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CGSize;
use objc2_metal::{MTL4CommandQueue, MTLDrawable};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};

impl Device {
    pub fn create_swapchain(
        &self,
        queue: u32,
        layer: Retained<CAMetalLayer>,
        info: &SwapchainInfo,
    ) -> Result<Swapchain, Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        let images = info.images;
        check!(
            (2..=3).contains(&images),
            "create_swapchain images {images} is outside 2..=3"
        );
        layer.setDevice(Some(&self.raw));
        layer.setPixelFormat(info.format.mtl());
        layer.setDrawableSize(CGSize {
            width: info.width as f64,
            height: info.height as f64,
        });
        layer.setMaximumDrawableCount(images as usize);
        layer.setFramebufferOnly(info.usage == TextureUsage::COLOR);
        layer.setDisplaySyncEnabled(info.vsync);
        self.queues[queue]
            .raw
            .addResidencySet(&layer.residencySet());
        Ok(self.res.swapchains.insert(SwapchainObject {
            layer,
            queue,
            format: info.format,
            current: Default::default(),
            textures: Default::default(),
        }))
    }

    pub fn destroy_swapchain(&self, swapchain: Swapchain) {
        let s = self.res.swapchains.remove(swapchain);
        for (_, texture) in s.textures.into_inner().unwrap() {
            self.destroy_texture(texture);
        }
        self.queues[s.queue]
            .raw
            .removeResidencySet(&s.layer.residencySet());
    }

    pub fn acquire_texture(&self, swapchain: Swapchain) -> Result<Texture, Error> {
        autoreleasepool(|_| {
            let s = self.res.swapchains.get(swapchain);
            let mut current = s.current.lock().unwrap();
            check!(
                current.is_none(),
                "acquire_texture on {swapchain} before submit_and_present presented the last one"
            );
            let drawable = s
                .layer
                .nextDrawable()
                .ok_or_else(|| Error::Driver("nextDrawable timed out".into()))?;
            let raw = drawable.texture();
            let mut textures = s.textures.lock().unwrap();
            let known = textures
                .iter()
                .find(|(t, _)| Retained::as_ptr(t) == Retained::as_ptr(&raw));
            let texture = match known {
                Some(&(_, texture)) => texture,
                None => {
                    let texture = self.import_texture(raw.clone(), s.format)?;
                    textures.push((raw, texture));
                    texture
                }
            };
            *current = Some((drawable, texture));
            Ok(texture)
        })
    }

    pub fn submit_and_present<'d>(
        &self,
        swapchain: Swapchain,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), Error> {
        let s = self.res.swapchains.get(swapchain);
        let current = s.current.lock().unwrap().take();
        let Some((drawable, _)) = current else {
            panic!("submit_and_present on {swapchain} without acquire_texture");
        };
        let drawable = ProtocolObject::<dyn MTLDrawable>::from_ref(&*drawable);
        self.submit_with(commands, waits, signals, Some((s.queue, drawable)))
    }
}
