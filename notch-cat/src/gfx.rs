// 그리기 바탕: D3D11 → DXGI 합성 스왑체인 → DirectComposition(투명) + Direct2D + DirectWrite.
use windows::core::{w, Interface, Result, PCWSTR};
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

pub type Rgb = (f32, f32, f32);

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Gfx {
    pub dc: ID2D1DeviceContext,
    swap: IDXGISwapChain1,
    _dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    dw: IDWriteFactory,
    d2f: ID2D1Factory1,
    brush: ID2D1SolidColorBrush,
    wic: IWICImagingFactory,
    pub f_title: IDWriteTextFormat,
    pub f_body: IDWriteTextFormat,
    pub f_lyric: IDWriteTextFormat,
    pub f_sub: IDWriteTextFormat,
    pub f_time: IDWriteTextFormat,
    pub f_tiny: IDWriteTextFormat,
}

unsafe fn font(dw: &IDWriteFactory, name: PCWSTR, size: f32, bold: bool) -> Result<IDWriteTextFormat> {
    let weight = if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL };
    let f = dw.CreateTextFormat(name, None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("ko-kr"))?;
    f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
    let sign = dw.CreateEllipsisTrimmingSign(&f)?;
    f.SetTrimming(&DWRITE_TRIMMING { granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER, delimiter: 0, delimiterCount: 0 }, &sign)?;
    Ok(f)
}

impl Gfx {
    /// dpi: 96 × (화면 배율 × 넓은 모니터 키움). 이후 좌표는 전부 논리 단위다.
    pub unsafe fn new(hwnd: HWND, wpx: u32, hpx: u32, dpi: f32) -> Result<Self> {
        let mut dev: Option<ID3D11Device> = None;
        let mut made = Err(windows::core::Error::empty());
        // 그래픽 카드가 없으면(빌드 서버) 소프트웨어(WARP)로.
        for ty in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
            made = D3D11CreateDevice(None, ty, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut dev), None, None);
            if made.is_ok() {
                break;
            }
        }
        made?;
        let dxgi: IDXGIDevice = dev.unwrap().cast()?;
        let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: wpx,
            Height: hpx,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            ..Default::default()
        };
        let swap = factory.CreateSwapChainForComposition(&dxgi, &desc, None)?;

        let d2f: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let dc = d2f.CreateDevice(&dxgi)?.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        let surface: IDXGISurface = swap.GetBuffer(0)?;
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: dpi,
            dpiY: dpi,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            ..Default::default()
        };
        let bmp = dc.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
        dc.SetTarget(&bmp);
        dc.SetDpi(dpi, dpi);
        dc.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE); // 투명 바탕에선 ClearType 이 안 된다

        let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
        let visual = dcomp.CreateVisual()?;
        visual.SetContent(&swap)?;
        target.SetRoot(&visual)?;
        dcomp.Commit()?;

        let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let brush = dc.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let mg = w!("Malgun Gothic");
        Ok(Gfx {
            f_title: font(&dw, mg, 17.0, true)?,
            f_body: font(&dw, mg, 13.0, false)?,
            f_lyric: font(&dw, mg, 15.0, true)?,
            f_sub: font(&dw, mg, 13.0, false)?,
            f_time: font(&dw, mg, 12.5, true)?,
            f_tiny: font(&dw, mg, 10.0, false)?,
            dc,
            d2f,
            swap,
            _dcomp: dcomp,
            _target: target,
            _visual: visual,
            dw,
            brush,
            wic,
        })
    }

    unsafe fn color(&self, c: Rgb, a: f32) {
        self.brush.SetColor(&D2D1_COLOR_F { r: c.0, g: c.1, b: c.2, a: a.clamp(0.0, 1.0) });
    }

    pub unsafe fn begin(&self) {
        self.dc.BeginDraw();
        self.dc.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
    }

    /// 다 그린 뒤 다음 화면 갱신 때 내보낸다(모니터 주사율에 맞춰 기다린다).
    pub unsafe fn present(&self) -> Result<()> {
        self.dc.EndDraw(None, None)?;
        self.swap.Present(1, DXGI_PRESENT(0)).ok()
    }

    pub unsafe fn rrect(&self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Rgb, a: f32) {
        if w <= 0.0 || h <= 0.0 || a <= 0.0 {
            return;
        }
        self.color(c, a);
        let rr = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }, radiusX: r, radiusY: r };
        self.dc.FillRoundedRectangle(&rr, &self.brush);
    }

    /// 꽉 찬 다각형(재생 버튼 세모 등).
    pub unsafe fn poly(&self, pts: &[(f32, f32)], c: Rgb, a: f32) {
        let Ok(geo) = self.d2f.CreatePathGeometry() else { return };
        let Ok(sink) = geo.Open() else { return };
        sink.BeginFigure(D2D_POINT_2F { x: pts[0].0, y: pts[0].1 }, D2D1_FIGURE_BEGIN_FILLED);
        let rest: Vec<D2D_POINT_2F> = pts[1..].iter().map(|p| D2D_POINT_2F { x: p.0, y: p.1 }).collect();
        sink.AddLines(&rest);
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        let _ = sink.Close();
        self.color(c, a);
        self.dc.FillGeometry(&geo, &self.brush, None);
    }

    pub unsafe fn clip(&self, x: f32, y: f32, w: f32, h: f32) {
        self.dc.PushAxisAlignedClip(&D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }, D2D1_ANTIALIAS_MODE_ALIASED);
    }

    pub unsafe fn unclip(&self) {
        self.dc.PopAxisAlignedClip();
    }

    unsafe fn layout(&self, s: &str, f: &IDWriteTextFormat, w: f32) -> Option<IDWriteTextLayout> {
        let u: Vec<u16> = s.encode_utf16().collect();
        self.dw.CreateTextLayout(&u, f, w.max(1.0), 100.0).ok()
    }

    /// 한 줄 글자의 자연 너비.
    pub unsafe fn measure(&self, s: &str, f: &IDWriteTextFormat) -> f32 {
        let mut m = DWRITE_TEXT_METRICS::default();
        if let Some(l) = self.layout(s, f, 4000.0) {
            let _ = l.GetMetrics(&mut m);
        }
        m.width
    }

    /// 너비 w 안에 한 줄. 넘치면 말줄임.
    pub unsafe fn text(&self, s: &str, f: &IDWriteTextFormat, x: f32, y: f32, w: f32, c: Rgb, a: f32, al: Align) {
        if a <= 0.01 || s.is_empty() {
            return;
        }
        let Some(l) = self.layout(s, f, w) else { return };
        let _ = l.SetTextAlignment(match al {
            Align::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
            Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            Align::Right => DWRITE_TEXT_ALIGNMENT_TRAILING,
        });
        self.color(c, a);
        self.dc.DrawTextLayout(D2D_POINT_2F { x, y }, &l, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
    }

    /// 앨범 그림 파일 바이트(jpg·png) → 그릴 수 있는 그림.
    pub unsafe fn bitmap(&self, bytes: &[u8]) -> Result<ID2D1Bitmap> {
        let stream = self.wic.CreateStream()?;
        stream.InitializeFromMemory(bytes)?;
        let frame = self.wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)?.GetFrame(0)?;
        let conv = self.wic.CreateFormatConverter()?;
        conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeMedianCut)?;
        let rt: ID2D1RenderTarget = self.dc.cast()?;
        rt.CreateBitmapFromWicBitmap(&conv, None)
    }

    /// 그림을 정사각형으로 가운데만 잘라 둥근 모서리로.
    pub unsafe fn art(&self, bmp: &ID2D1Bitmap, x: f32, y: f32, size: f32, r: f32, a: f32) {
        if a <= 0.01 {
            return;
        }
        let rt: ID2D1RenderTarget = match self.dc.cast() {
            Ok(r) => r,
            Err(_) => return,
        };
        let sz = bmp.GetSize();
        let k = size / sz.width.min(sz.height).max(1.0);
        let bp = D2D1_BRUSH_PROPERTIES {
            opacity: a.clamp(0.0, 1.0),
            transform: Matrix3x2 { M11: k, M12: 0.0, M21: 0.0, M22: k, M31: x - (sz.width * k - size) / 2.0, M32: y - (sz.height * k - size) / 2.0 },
        };
        if let Ok(b) = rt.CreateBitmapBrush(bmp, None, Some(&bp)) {
            let rr = D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x, top: y, right: x + size, bottom: y + size }, radiusX: r, radiusY: r };
            self.dc.FillRoundedRectangle(&rr, &b);
        }
    }
}
