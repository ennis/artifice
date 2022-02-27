//! Global application singleton for win32
use crate::backend::Error;
use std::{
    ffi::OsString,
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};
use windows::{
    core::Interface,
    Win32::{
        Graphics::{
            Direct2D::{
                D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, D2D1_DEBUG_LEVEL_WARNING,
                D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_MULTI_THREADED,
            },
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_1},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device5, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_DEBUG,
                D3D11_SDK_VERSION,
            },
            DirectWrite::{DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED},
            Dxgi::{CreateDXGIFactory2, IDXGIDevice, IDXGIFactory3},
            Imaging::{CLSID_WICImagingFactory2, D2D::IWICImagingFactory2},
        },
        System::Com::{CoCreateInstance, CoInitialize, CLSCTX_INPROC_SERVER},
        UI::{
            Input::KeyboardAndMouse::GetDoubleClickTime,
            WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG},
        },
    },
};

macro_rules! sync_com_ptr_wrapper {
    ($wrapper:ident ( $iface:ident ) ) => {
        #[derive(Clone)]
        pub(crate) struct $wrapper(pub(crate) $iface);
        unsafe impl Sync for $wrapper {} // ok to send &I across threads
        unsafe impl Send for $wrapper {} // ok to send I across threads
        impl Deref for $wrapper {
            type Target = $iface;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

macro_rules! send_com_ptr_wrapper {
    ($wrapper:ident ( $iface:ident ) ) => {
        #[derive(Clone)]
        pub(crate) struct $wrapper(pub(crate) $iface);
        unsafe impl Send for $wrapper {} // ok to send I across threads
        impl Deref for $wrapper {
            type Target = $iface;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

// Thread safety notes: some services are thread-safe, some are not, and for some we don't know due to poor documentation.
// Additionally, some services should only be used on the "main" thread or the "UI" thread.
// There are different ways to ensure thread safety:
// 1. Mutex-wrap all services for which we have no information about thread-safety
// 2. Restrict access to services to the main thread
//
// Option 2 may seem harsh. Consider an application that layouts the GUI in parallel: it might want to access
// the text services (to measure a text string) simultaneously across several threads.
//
// FIXME this might be a bit too optimistic...

sync_com_ptr_wrapper! { D3D11Device(ID3D11Device5) }
sync_com_ptr_wrapper! { DXGIFactory3(IDXGIFactory3) }
sync_com_ptr_wrapper! { D2D1Factory1(ID2D1Factory1) }
sync_com_ptr_wrapper! { DWriteFactory(IDWriteFactory) }
sync_com_ptr_wrapper! { D2D1Device(ID2D1Device) }
sync_com_ptr_wrapper! { WICImagingFactory2(IWICImagingFactory2) }
send_com_ptr_wrapper! { D2D1DeviceContext(ID2D1DeviceContext) }

/// Encapsulates various global factories for windows.
///
// all of this must be either directly Sync, or wrapped in a mutex, or wrapped in a main-thread-only wrapper.
pub(crate) struct Application {
    pub(crate) gpu_device: Arc<graal::Device>,
    pub(crate) gpu_context: Mutex<graal::Context>,
    pub(crate) d3d11_device: D3D11Device, // thread safe
    pub(crate) dxgi_factory: DXGIFactory3,
}

impl Application {
    pub(crate) fn new() -> Result<Application, Error> {
        // --- Create the graal context (implying a vulkan instance and device)
        // FIXME technically we need the target surface so we can pick a device that can
        // render to it. However, on most systems, all available devices can render to window surfaces,
        // so skip that for now.
        let (gpu_device, gpu_context) = unsafe {
            // SAFETY: we don't pass a surface handle
            graal::create_device_and_context(None)
        };

        // ---------- DXGI Factory ----------

        // SAFETY: the paramters are valid
        let dxgi_factory = unsafe { DXGIFactory3(CreateDXGIFactory2::<IDXGIFactory3>(0).unwrap()) };

        // --- Enumerate adapters
        let mut adapters = Vec::new();
        unsafe {
            let mut i = 0;
            while let Ok(adapter) = dxgi_factory.EnumAdapters1(i) {
                adapters.push(adapter);
                i += 1;
            }
        };

        for adapter in adapters.iter() {
            let desc = unsafe { adapter.GetDesc1().unwrap() };

            use std::os::windows::ffi::OsStringExt;

            let name = &desc.Description[..];
            let name_len = name.iter().take_while(|&&c| c != 0).count();
            let name = OsString::from_wide(&desc.Description[..name_len])
                .to_string_lossy()
                .into_owned();
            tracing::info!(
                "DXGI adapter: name={}, LUID={:08x}{:08x}",
                name,
                desc.AdapterLuid.HighPart,
                desc.AdapterLuid.LowPart,
            );
        }

        // --- Create the D3D11 device and device context

        // This is needed for D2D stuff.
        // SAFETY: the parameters are valid
        let (d3d11_device, d3d11_device_context) = unsafe {
            let mut d3d11_device = None;
            let mut feature_level = D3D_FEATURE_LEVEL::default();
            let mut _d3d11_device_context = None;

            let feature_levels = [D3D_FEATURE_LEVEL_11_1];

            D3D11CreateDevice(
                // pAdapter:
                None,
                // DriverType:
                D3D_DRIVER_TYPE_HARDWARE,
                // Software:
                None,
                // Flags:
                D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG,
                // pFeatureLevels:
                feature_levels.as_ptr(),
                // FeatureLevels:
                1,
                // SDKVersion
                D3D11_SDK_VERSION,
                // ppDevice:
                &mut d3d11_device,
                // pFeatureLevel:
                &mut feature_level,
                // ppImmediateContext:
                &mut _d3d11_device_context,
            )?;

            tracing::info!("Direct3D feature level: {:?}", feature_level);

            (
                D3D11Device(d3d11_device.unwrap().cast::<ID3D11Device5>().unwrap()),
                _d3d11_device_context.unwrap(),
            )
        };

        // SAFETY: pointers should be non-null if D3D11CreateDevice succeeds

        let app = Application {
            gpu_device,
            gpu_context: Mutex::new(gpu_context),
            d3d11_device,
            dxgi_factory,
        };

        Ok(app)
    }

    /// Returns the system double click time in milliseconds.
    pub fn double_click_time(&self) -> Duration {
        unsafe {
            let ms = GetDoubleClickTime();
            Duration::from_millis(ms as u64)
        }
    }

    /// Enters the main event loop.
    pub(crate) fn run(&self) {
        unsafe {
            let mut msg = MaybeUninit::<MSG>::uninit();
            loop {
                let result = GetMessageW(msg.as_mut_ptr(), None, 0, 0);
                let msg = msg.assume_init();
                if !result.as_bool() {
                    break;
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
