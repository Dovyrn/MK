#[cfg(all(windows, any(feature = "vk", feature = "gl")))]
use crate::host;

#[cfg(not(all(windows, any(feature = "vk", feature = "gl"))))]
pub fn test() {
    println!("D3D11 sharing needs Windows and Vulkan or GL");
}

#[cfg(all(windows, any(feature = "vk", feature = "gl")))]
pub fn test() {
    use mk::{ColorAttachment, Error, Format, RenderPassInfo, TextureInfo, TextureUsage};

    let device = host::device(1);
    let luid = device.device_luid();
    if luid == 0 {
        println!("the device reports no LUID");
        return;
    }
    let size = 16;
    let d3d = d3d::D3d::new(luid, size);
    let info = TextureInfo {
        format: Format::Rgba8Unorm,
        width: size,
        height: size,
        usage: TextureUsage::COLOR,
        ..Default::default()
    };
    let texture = match device.import_shared_texture(d3d.texture_handle.0 as u64, &info) {
        Err(Error::Unsupported(e)) => {
            println!("{e}");
            return;
        }
        result => result.unwrap(),
    };
    let timeline = device
        .import_shared_timeline(d3d.fence_handle.0 as u64)
        .unwrap();
    let pool = device.create_command_pool(0).unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    #[cfg(feature = "vk")]
    c.acquire_shared(texture, mk::Stage::COLOR);
    let pass = c
        .begin_render_pass(&RenderPassInfo {
            width: size,
            height: size,
            colors: &[ColorAttachment {
                texture,
                clear: [1.0, 0.2, 0.6, 1.0],
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.end();
    #[cfg(feature = "vk")]
    c.release_shared(texture, mk::Stage::COLOR);
    c.end().unwrap();
    #[cfg(feature = "vk")]
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    #[cfg(feature = "gl")]
    {
        device.submit([c], &[], &[]).unwrap();
        device.signal_shared(&timeline, 1, &[texture]);
    }
    let pixels = d3d.read(1);
    assert!(
        pixels.iter().all(|p| *p == [255, 51, 153, 255]),
        "{:?}",
        pixels[0]
    );
    device.destroy_texture(texture);
    device.destroy_timeline(timeline);
    device.destroy_command_pool(pool);
    println!("D3D11 read {size}x{size} texels MK cleared, ordered by a shared D3D12 fence");
}

#[cfg(all(windows, any(feature = "vk", feature = "gl")))]
mod d3d {
    use windows::Win32::Foundation::{CloseHandle, GENERIC_ALL, HANDLE, HMODULE, LUID};
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::*;
    use windows::core::{Interface, PCWSTR};

    pub struct D3d {
        context: ID3D11DeviceContext4,
        texture: ID3D11Texture2D,
        staging: ID3D11Texture2D,
        fence: ID3D11Fence,
        size: u32,
        pub texture_handle: HANDLE,
        pub fence_handle: HANDLE,
    }

    impl D3d {
        pub fn new(luid: u64, size: u32) -> Self {
            unsafe {
                let factory: IDXGIFactory4 = CreateDXGIFactory1().unwrap();
                let luid = LUID {
                    LowPart: luid as u32,
                    HighPart: (luid >> 32) as i32,
                };
                let adapter: IDXGIAdapter = factory.EnumAdapterByLuid(luid).unwrap();
                let mut device = None;
                let mut context = None;
                D3D11CreateDevice(
                    &adapter,
                    D3D_DRIVER_TYPE_UNKNOWN,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_FLAG(0),
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
                .unwrap();
                let device: ID3D11Device5 = device.unwrap().cast().unwrap();
                let context: ID3D11DeviceContext4 = context.unwrap().cast().unwrap();
                let mut desc = D3D11_TEXTURE2D_DESC {
                    Width: size,
                    Height: size,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: (D3D11_RESOURCE_MISC_SHARED.0
                        | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0)
                        as u32,
                };
                let mut texture = None;
                device
                    .CreateTexture2D(&desc, None, Some(&mut texture))
                    .unwrap();
                let texture = texture.unwrap();
                let resource: IDXGIResource1 = texture.cast().unwrap();
                let access = DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE;
                let texture_handle = resource
                    .CreateSharedHandle(None, access.0, PCWSTR::null())
                    .unwrap();
                desc.Usage = D3D11_USAGE_STAGING;
                desc.BindFlags = 0;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                desc.MiscFlags = 0;
                let mut staging = None;
                device
                    .CreateTexture2D(&desc, None, Some(&mut staging))
                    .unwrap();
                let mut fence: Option<ID3D11Fence> = None;
                device
                    .CreateFence(0, D3D11_FENCE_FLAG_SHARED, &mut fence)
                    .unwrap();
                let fence = fence.unwrap();
                let fence_handle = fence
                    .CreateSharedHandle(None, GENERIC_ALL.0, PCWSTR::null())
                    .unwrap();
                Self {
                    context,
                    texture,
                    staging: staging.unwrap(),
                    fence,
                    size,
                    texture_handle,
                    fence_handle,
                }
            }
        }

        pub fn read(&self, value: u64) -> Vec<[u8; 4]> {
            unsafe {
                self.context.Wait(&self.fence, value).unwrap();
                self.context.CopyResource(&self.staging, &self.texture);
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                self.context
                    .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                    .unwrap();
                let mut pixels = Vec::new();
                for y in 0..self.size as usize {
                    let row = mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize);
                    for x in 0..self.size as usize {
                        let p = row.add(x * 4);
                        pixels.push([*p, *p.add(1), *p.add(2), *p.add(3)]);
                    }
                }
                self.context.Unmap(&self.staging, 0);
                pixels
            }
        }
    }

    impl Drop for D3d {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.texture_handle).unwrap();
                CloseHandle(self.fence_handle).unwrap();
            }
        }
    }
}
