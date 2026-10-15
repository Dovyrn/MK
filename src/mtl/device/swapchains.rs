use super::Device;
use crate::mtl::swapchain::{Presented, State, SwapchainObject};
use crate::{
    ColorSpace, CommandBuffer, Error, Format, PresentMode, SurfaceCaps, Swapchain, SwapchainInfo,
    Texture, TextureUsage, Timeline,
};
use block2::RcBlock;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CGSize;
use objc2_core_graphics::{
    CGColorSpace, kCGColorSpaceDisplayP3, kCGColorSpaceExtendedLinearSRGB,
    kCGColorSpaceITUR_2100_PQ, kCGColorSpaceSRGB,
};
use objc2_metal::{MTL4CommandQueue, MTLDrawable};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

impl Device {
    pub fn surface_caps(&self, layer: &CAMetalLayer) -> Result<SurfaceCaps, Error> {
        let size = layer.drawableSize();
        Ok(SurfaceCaps {
            min_images: 2,
            max_images: 3,
            width: size.width as u32,
            height: size.height as u32,
            mailbox: false,
            immediate: cfg!(target_os = "macos"),
        })
    }

    pub fn surface_format(
        &self,
        _: &CAMetalLayer,
        format: Format,
        space: ColorSpace,
    ) -> Result<bool, Error> {
        let deep = matches!(format, Format::Rgba16Float);
        let wide = matches!(format, Format::Rgb10a2Unorm | Format::Bgr10a2Unorm);
        let narrow = matches!(format, Format::Bgra8Unorm | Format::Bgra8Srgb);
        Ok(match space {
            ColorSpace::Srgb | ColorSpace::DisplayP3 => deep || wide || narrow,
            ColorSpace::ExtendedSrgb => deep,
            ColorSpace::Hdr10 => deep || wide,
        })
    }

    fn configure(&self, layer: &CAMetalLayer, info: &SwapchainInfo) -> Result<(), Error> {
        if cfg!(feature = "validation") {
            let caps = self.surface_caps(layer)?;
            let images = info.images;
            check!(
                (caps.min_images..=caps.max_images).contains(&images),
                "swapchain images {images} is outside the layer's 2..=3"
            );
            let (format, space) = (info.format, info.color_space);
            check!(
                self.surface_format(layer, format, space)?,
                "the layer does not take {format} in {space:?}"
            );
            let mode = info.mode;
            check!(
                mode == PresentMode::Fifo || mode == PresentMode::Immediate && caps.immediate,
                "the layer has no {mode:?} present mode"
            );
        }
        let name = unsafe {
            match info.color_space {
                ColorSpace::Srgb => kCGColorSpaceSRGB,
                ColorSpace::DisplayP3 => kCGColorSpaceDisplayP3,
                ColorSpace::ExtendedSrgb => kCGColorSpaceExtendedLinearSRGB,
                ColorSpace::Hdr10 => kCGColorSpaceITUR_2100_PQ,
            }
        };
        layer.setDevice(Some(&self.raw));
        layer.setPixelFormat(info.format.mtl());
        layer.setColorspace(CGColorSpace::with_name(Some(name)).as_deref());
        layer.setWantsExtendedDynamicRangeContent(matches!(
            info.color_space,
            ColorSpace::ExtendedSrgb | ColorSpace::Hdr10
        ));
        layer.setDrawableSize(CGSize {
            width: info.width as f64,
            height: info.height as f64,
        });
        layer.setMaximumDrawableCount(info.images as usize);
        layer.setFramebufferOnly(info.usage == TextureUsage::COLOR);
        layer.setAllowsNextDrawableTimeout(false);
        #[cfg(target_os = "macos")]
        layer.setDisplaySyncEnabled(info.mode == PresentMode::Fifo);
        Ok(())
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

    fn forget_textures(&self, state: &mut State) {
        for (_, texture) in state.textures.drain(..) {
            self.destroy_texture(texture);
        }
    }

    pub fn create_swapchain(
        &self,
        queue: u32,
        layer: Retained<CAMetalLayer>,
        info: &SwapchainInfo,
    ) -> Result<Swapchain, Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        self.configure(&layer, info)?;
        self.queues[queue]
            .raw
            .addResidencySet(&layer.residencySet());
        Ok(self.res.swapchains.insert(SwapchainObject {
            layer,
            queue,
            state: Mutex::new(State {
                format: info.format,
                current: None,
                textures: Vec::new(),
                count: 0,
                mark: None,
            }),
            presented: Arc::default(),
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
        self.configure(&s.layer, info)?;
        self.forget_textures(&mut state);
        state.format = info.format;
        Ok(())
    }

    pub fn destroy_swapchain(&self, swapchain: Swapchain) {
        let s = self.res.swapchains.remove(swapchain);
        let mut state = s.state.into_inner().unwrap();
        self.check_idle("destroy_swapchain", swapchain, s.queue, &state);
        self.forget_textures(&mut state);
        self.queues[s.queue]
            .raw
            .removeResidencySet(&s.layer.residencySet());
    }

    pub fn acquire_texture(&self, swapchain: Swapchain) -> Result<Texture, Error> {
        autoreleasepool(|_| {
            let s = self.res.swapchains.get(swapchain);
            let mut state = s.state.lock().unwrap();
            check!(
                state.current.is_none(),
                "acquire_texture on {swapchain} before submit_and_present presented the last one"
            );
            let drawable = s
                .layer
                .nextDrawable()
                .ok_or_else(|| Error::OutOfDate("nextDrawable returned no drawable".into()))?;
            let raw = drawable.texture();
            let known = state
                .textures
                .iter()
                .find(|(t, _)| Retained::as_ptr(t) == Retained::as_ptr(&raw));
            let texture = match known {
                Some(&(_, texture)) => texture,
                None => {
                    let texture = self.import_texture(raw.clone(), state.format)?;
                    self.res.table.presentable.set(texture, Default::default());
                    state.textures.push((raw, texture));
                    texture
                }
            };
            state.current = Some((drawable, texture));
            self.res.table.acquire(texture, true);
            Ok(texture)
        })
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
        let Some((drawable, texture)) = state.current.take() else {
            panic!("submit_and_present on {swapchain} without acquire_texture");
        };
        state.count += 1;
        state.mark = Some(self.queues[s.queue].idles.load(Ordering::Relaxed));
        let id = state.count;
        drop(state);
        self.res.table.acquire(texture, false);
        let presented = s.presented.clone();
        let handler = RcBlock::new(move |_: NonNull<ProtocolObject<dyn MTLDrawable>>| {
            let mut count = presented.count.lock().unwrap();
            *count = (*count).max(id);
            presented.changed.notify_all();
        });
        unsafe { drawable.addPresentedHandler(RcBlock::as_ptr(&handler)) };
        let drawable = ProtocolObject::<dyn MTLDrawable>::from_ref(&*drawable);
        self.submit_with(commands, waits, signals, Some((s.queue, drawable)))?;
        Ok(false)
    }

    pub fn wait_presented(
        &self,
        swapchain: Swapchain,
        present: u64,
        ms: u64,
    ) -> Result<bool, Error> {
        let presented: &Presented = &self.res.swapchains.get(swapchain).presented;
        let count = presented.count.lock().unwrap();
        let (count, _) = presented
            .changed
            .wait_timeout_while(count, Duration::from_millis(ms), |c| *c < present)
            .unwrap();
        Ok(*count >= present)
    }
}
