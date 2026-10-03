use super::{RENDER, Recorder, origin, size};
use crate::mtl::texture::Texture;
use crate::{Command, dimension, filter, format};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::{
    MTL4ArgumentTable, MTL4CommandBuffer, MTL4CommandEncoder, MTL4ComputeCommandEncoder,
    MTL4RenderCommandEncoder, MTL4RenderPassDescriptor, MTL4VisibilityOptions, MTLClearColor,
    MTLLoadAction, MTLPrimitiveType, MTLRenderPassAttachmentDescriptor, MTLRenderStages,
    MTLSamplerState, MTLScissorRect, MTLStages, MTLStoreAction, MTLTexture, MTLTextureType,
    MTLViewport,
};

type Encoder = Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>;

type View = Retained<ProtocolObject<dyn MTLTexture>>;

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
                        &s.raw,
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
        Ok(())
    }

    fn view(
        texture: &ProtocolObject<dyn MTLTexture>,
        kind: MTLTextureType,
        mip: u32,
        layer: u32,
    ) -> Result<View, &'static str> {
        unsafe {
            texture
                .newTextureViewWithPixelFormat_textureType_levels_slices(
                    texture.pixelFormat(),
                    kind,
                    NSRange::new(mip as usize, 1),
                    NSRange::new(layer as usize, 1),
                )
                .ok_or("blit view allocation failed")
        }
    }

    fn draw(
        &mut self,
        view: View,
        from: [f32; 7],
        (target, mip, slice, plane): (&Texture, u32, u32, u32),
        to: [u32; 4],
        filter: u32,
    ) -> Result<(), &'static str> {
        let volume = view.textureType() == MTLTextureType::Type3D;
        let state = self.blit.state(self.device, target.format, volume)?;
        let [x, y, width, height] = to;
        let rects = [&from[..4], &to.map(|v| v as f32)[..], &from[4..], &[0.0]].concat();
        let bytes: Vec<u8> = rects.iter().flat_map(|v| v.to_le_bytes()).collect();
        let address = self.frame.push(self.device, self.residency, &bytes)?;
        let table = &self.blit.table;
        unsafe {
            table.setAddress_atIndex(address, 0);
            table.setTexture_atIndex(view.gpuResourceID(), 0);
            table.setSamplerState_atIndex(self.blit.sampler(filter).gpuResourceID(), 0);
        }
        let raw = &target.raw;
        let w = (raw.width() >> mip).max(1) as u32;
        let h = (raw.height() >> mip).max(1) as u32;
        let whole = x == 0 && y == 0 && width == w && height == h;
        let desc = MTL4RenderPassDescriptor::new();
        desc.setRenderTargetWidth(w as usize);
        desc.setRenderTargetHeight(h as usize);
        unsafe { desc.setDefaultRasterSampleCount(1) };
        let load = if whole {
            MTLLoadAction::DontCare
        } else {
            MTLLoadAction::Load
        };
        let color = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
        attach(&color, raw, (mip, slice, plane), load);
        let encoder = self.pass(&desc)?;
        let stages = MTLRenderStages::Vertex | MTLRenderStages::Fragment;
        encoder.setArgumentTable_atStages(table, stages);
        encoder.setRenderPipelineState(&state);
        encoder.setViewport(MTLViewport {
            originX: x as f64,
            originY: y as f64,
            width: width as f64,
            height: height as f64,
            znear: 0.0,
            zfar: 1.0,
        });
        encoder.setScissorRect(MTLScissorRect {
            x: x as usize,
            y: y as usize,
            width: width as usize,
            height: height as usize,
        });
        unsafe { encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3) };
        Self::close(encoder);
        self.frame.keep(view);
        Ok(())
    }

    pub(super) fn generate_mips(&mut self, texture: u32) -> Result<(), &'static str> {
        let res = self.res;
        let info = *res.table.textures.get(texture);
        let t = res.textures.get(texture);
        let volume = info.dimension == dimension::D3;
        let kind = if volume {
            MTLTextureType::Type3D
        } else {
            MTLTextureType::Type2D
        };
        let size = |m: u32| [info.width, info.height, info.depth].map(|n| (n >> m).max(1) as f32);
        let open = self.suspend()?;
        for m in 1..info.mips {
            let [w, h, _] = size(m - 1);
            let [x, y, d] = size(m);
            let slices = if volume { d as u32 } else { info.layers };
            for l in 0..slices {
                let (layer, plane) = if volume { (0, l) } else { (l, 0) };
                let view = Self::view(&t.raw, kind, m - 1, layer)?;
                let depth = (l as f32 + 0.5) / d;
                let from = [0.0, 0.0, w, h, w, h, depth];
                let to = [0, 0, x as u32, y as u32];
                self.draw(view, from, (t, m, layer, plane), to, filter::LINEAR)?;
            }
        }
        self.resume(open)
    }

    pub(super) fn blit_texture(&mut self, c: Command) -> Result<(), &'static str> {
        let Command::BlitTexture {
            src,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_width,
            src_height,
            dst,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_width,
            dst_height,
            filter,
            ..
        } = c
        else {
            return Ok(());
        };
        let res = self.res;
        let (s, d) = (res.textures.get(src), res.textures.get(dst));
        let info = res.table.textures.get(src);
        let view = Self::view(&s.raw, MTLTextureType::Type2D, src_mip, src_layer)?;
        let [w, h] = [info.width, info.height].map(|n| (n >> src_mip).max(1) as f32);
        let [x, y, sw, sh] = [src_x, src_y, src_width, src_height].map(|v| v as f32);
        let to = [dst_x, dst_y, dst_width, dst_height];
        let open = self.suspend()?;
        let from = [x, y, sw, sh, w, h, 0.0];
        self.draw(view, from, (d, dst_mip, dst_layer, 0), to, filter)?;
        self.resume(open)
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
