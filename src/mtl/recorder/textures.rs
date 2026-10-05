use super::{RENDER, Recorder, origin, size};
use crate::{Command, dimension, format};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandBuffer, MTL4CommandEncoder, MTL4ComputeCommandEncoder, MTL4RenderCommandEncoder,
    MTL4RenderPassDescriptor, MTL4VisibilityOptions, MTLClearColor, MTLLoadAction,
    MTLRenderPassAttachmentDescriptor, MTLStages, MTLStoreAction, MTLTexture,
};

type Encoder = Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>;

fn attach(
    a: &MTLRenderPassAttachmentDescriptor,
    texture: &ProtocolObject<dyn MTLTexture>,
    (mip, slice, plane): (u32, u32, u32),
    load: MTLLoadAction,
) {
    a.setTexture(Some(texture));
    a.setLevel(mip as usize);
    a.setSlice(slice as usize);
    a.setDepthPlane(plane as usize);
    a.setLoadAction(load);
    a.setStoreAction(MTLStoreAction::Store);
}

fn rest(n: u32, base: u32, total: u32) -> u32 {
    if n == !0 { total - base } else { n }
}

impl Recorder<'_> {
    fn suspend(&mut self) -> Result<bool, &'static str> {
        let open = self.compute.is_some();
        if open {
            self.step(Command::EndCompute)?;
        }
        Ok(open)
    }

    fn resume(&mut self, open: bool) -> Result<(), &'static str> {
        if open {
            self.step(Command::BeginCompute)?;
        }
        Ok(())
    }

    fn pass(&mut self, desc: &MTL4RenderPassDescriptor) -> Result<Encoder, &'static str> {
        let encoder = self
            .buffer
            .renderCommandEncoderWithDescriptor(desc)
            .ok_or("render encoder allocation failed")?;
        self.opened(ProtocolObject::from_ref(&*encoder), RENDER);
        encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(
            MTLStages::All,
            RENDER,
            MTL4VisibilityOptions::None,
        );
        Ok(encoder)
    }

    fn close(encoder: Encoder) {
        encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(
            RENDER,
            MTLStages::All,
            MTL4VisibilityOptions::None,
        );
        encoder.endEncoding();
    }

    pub(super) fn copy_texture(&mut self, c: Command) -> Result<(), &'static str> {
        let Command::CopyTexture {
            src,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_z,
            dst,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_z,
            width,
            height,
            depth,
            layers,
            ..
        } = c
        else {
            return Ok(());
        };
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let cast = if s.raw.pixelFormat() == d.raw.pixelFormat() {
            None
        } else {
            let view = s.raw.newTextureViewWithPixelFormat(d.raw.pixelFormat());
            Some(view.ok_or("copy view allocation failed")?)
        };
        let source = cast.as_deref().unwrap_or(&*s.raw);
        let extent = size(width, height, depth);
        let (from, to) = (origin(src_x, src_y, src_z), origin(dst_x, dst_y, dst_z));
        let open = self.compute.is_none();
        if open {
            self.step(Command::BeginCompute)?;
        }
        for i in 0..layers as usize {
            unsafe {
                self.compute()
                    .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        source,
                        src_layer as usize + i,
                        src_mip as usize,
                        from,
                        extent,
                        &d.raw,
                        dst_layer as usize + i,
                        dst_mip as usize,
                        to,
                    )
            }
        }
        if open {
            self.step(Command::EndCompute)?;
        }
        if let Some(view) = cast {
            self.frame.keep(view);
        }
        Ok(())
    }

    pub(super) fn clear_texture(&mut self, c: Command) -> Result<(), &'static str> {
        let Command::ClearTexture {
            texture,
            base_mip,
            mips,
            base_layer,
            layers,
            r,
            g,
            b,
            a,
            depth,
            stencil,
            ..
        } = c
        else {
            return Ok(());
        };
        let res = self.res;
        let t = res.textures.get(texture);
        let info = *res.table.textures.get(texture);
        let mips = rest(mips, base_mip, info.mips);
        let layers = rest(layers, base_layer, info.layers);
        let volume = info.dimension == dimension::D3;
        let open = self.suspend()?;
        for m in base_mip..base_mip + mips {
            let [w, h, d] = [info.width, info.height, info.depth].map(|n| (n >> m).max(1));
            let (slice, count) = if volume { (0, d) } else { (base_layer, layers) };
            let desc = MTL4RenderPassDescriptor::new();
            desc.setRenderTargetWidth(w as usize);
            desc.setRenderTargetHeight(h as usize);
            desc.setRenderTargetArrayLength(count as usize);
            unsafe { desc.setDefaultRasterSampleCount(t.raw.sampleCount()) };
            let at = (m, slice, 0);
            let clear = MTLLoadAction::Clear;
            if (format::D16_UNORM..=format::S8_UINT).contains(&info.format) {
                if info.format != format::S8_UINT {
                    let slot = desc.depthAttachment();
                    attach(&slot, &t.raw, at, clear);
                    slot.setClearDepth(depth as f64);
                }
                if matches!(
                    info.format,
                    format::D24_UNORM_S8 | format::D32_FLOAT_S8 | format::S8_UINT
                ) {
                    let slot = desc.stencilAttachment();
                    attach(&slot, &t.raw, at, clear);
                    slot.setClearStencil(stencil);
                }
            } else {
                let slot = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
                attach(&slot, &t.raw, at, clear);
                slot.setClearColor(MTLClearColor {
                    red: r as f64,
                    green: g as f64,
                    blue: b as f64,
                    alpha: a as f64,
                });
            }
            Self::close(self.pass(&desc)?);
        }
        self.resume(open)
    }
}
