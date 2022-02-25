use crate::{
    count_until_utf16, count_utf16, factory::dwrite_factory, formatted_text::FormattedText, Attribute, Error,
    FontStyle, FontWeight, ToDirectWrite, ToWString,
};
use kyute_common::{Data, Point, PointI, Rect, RectI, Size, SizeI, Transform, UnknownUnit};
use std::{
    ffi::c_void,
    mem::MaybeUninit,
    ops::Range,
    ptr,
    sync::{Arc, Mutex},
};
use windows::{
    core::{implement, IUnknown, HRESULT, PCWSTR},
    Win32::{
        Foundation::{BOOL, ERROR_INSUFFICIENT_BUFFER, RECT},
        Graphics::{
            Direct2D::Common::{
                ID2D1SimplifiedGeometrySink, D2D1_BEZIER_SEGMENT, D2D1_FIGURE_BEGIN, D2D1_FIGURE_END, D2D1_FILL_MODE,
                D2D1_PATH_SEGMENT, D2D_POINT_2F,
            },
            DirectWrite::{
                DWRITE_TEXTURE_ALIASED_1x1, DWRITE_TEXTURE_CLEARTYPE_3x1, IDWriteFactory7, IDWriteFontFace,
                IDWriteGlyphRunAnalysis, IDWriteInlineObject, IDWritePixelSnapping_Impl, IDWriteTextFormat3,
                IDWriteTextLayout, IDWriteTextLayout3, IDWriteTextRenderer, IDWriteTextRenderer_Impl,
                DWRITE_FONT_STRETCH, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_GLYPH_RUN, DWRITE_GLYPH_RUN_DESCRIPTION, DWRITE_HIT_TEST_METRICS, DWRITE_LINE_METRICS,
                DWRITE_MATRIX, DWRITE_MEASURING_MODE, DWRITE_MEASURING_MODE_NATURAL,
                DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL, DWRITE_STRIKETHROUGH, DWRITE_TEXTURE_TYPE,
                DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_UNDERLINE,
            },
        },
    },
};

/// A laid-out block of text.
pub struct Paragraph {
    layout: IDWriteTextLayout,
    text: Arc<str>,
}

/// Returns (start, len).
fn to_dwrite_text_range(text: &str, range: Range<usize>) -> DWRITE_TEXT_RANGE {
    let utf16_start = count_utf16(&text[0..range.start]);
    let utf16_len = count_utf16(&text[range.start..range.end]);

    DWRITE_TEXT_RANGE {
        startPosition: utf16_start as u32,
        length: utf16_len as u32,
    }
}

/// Text hit-test metrics.
#[derive(Copy, Clone, Debug, PartialEq, Data)]
pub struct HitTestMetrics {
    /// Text position in UTF-8 code units (bytes).
    pub text_position: usize,
    pub length: usize,
    pub bounds: Rect,
}

impl HitTestMetrics {
    pub(crate) fn from_dwrite(metrics: &DWRITE_HIT_TEST_METRICS, text: &str) -> HitTestMetrics {
        // convert utf16 code unit offset to utf8
        //dbg!(metrics.textPosition);
        let text_position = count_until_utf16(text, metrics.textPosition as usize);
        let length = count_until_utf16(&text[text_position..], metrics.length as usize);
        HitTestMetrics {
            text_position,
            length,
            bounds: Rect::new(
                Point::new(metrics.left as f64, metrics.top as f64),
                Size::new(metrics.width as f64, metrics.height as f64),
            ),
        }
    }
}

/// Return value of [TextLayout::hit_test_point].
#[derive(Copy, Clone, Debug, PartialEq, Data)]
pub struct HitTestPoint {
    pub is_trailing_hit: bool,
    pub metrics: HitTestMetrics,
}

/// Return value of [TextLayout::hit_test_text_position].
#[derive(Copy, Clone, Debug, PartialEq, Data)]
pub struct HitTestTextPosition {
    pub point: Point,
    pub metrics: HitTestMetrics,
}

#[derive(Copy, Clone, Debug, PartialEq, Data)]
pub struct TextMetrics {
    pub bounds: Rect,
    pub width_including_trailing_whitespace: f32,
    pub line_count: u32,
    pub max_bidi_reordering_depth: u32,
}

impl From<DWRITE_TEXT_METRICS> for TextMetrics {
    fn from(m: DWRITE_TEXT_METRICS) -> Self {
        TextMetrics {
            bounds: Rect::new(
                Point::new(m.left as f64, m.top as f64),
                Size::new(m.width as f64, m.height as f64),
            ),
            width_including_trailing_whitespace: m.widthIncludingTrailingWhitespace,
            max_bidi_reordering_depth: m.maxBidiReorderingDepth,
            line_count: m.lineCount,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Data)]
pub struct LineMetrics {
    pub length: u32,
    pub trailing_whitespace_length: u32,
    pub newline_length: u32,
    pub height: f64,
    pub baseline: f64,
    pub is_trimmed: bool,
}

impl From<DWRITE_LINE_METRICS> for LineMetrics {
    fn from(m: DWRITE_LINE_METRICS) -> Self {
        LineMetrics {
            length: m.length,
            trailing_whitespace_length: m.trailingWhitespaceLength,
            newline_length: m.newlineLength,
            height: m.height as f64,
            baseline: m.baseline as f64,
            is_trimmed: m.isTrimmed.as_bool(),
        }
    }
}

impl Paragraph {
    pub fn hit_test_point(&self, point: Point) -> Result<HitTestPoint, Error> {
        unsafe {
            let mut is_trailing_hit = false.into();
            let mut is_inside = false.into();
            let mut metrics = MaybeUninit::<DWRITE_HIT_TEST_METRICS>::uninit();
            self.layout.HitTestPoint(
                point.x as f32,
                point.y as f32,
                &mut is_trailing_hit,
                &mut is_inside,
                metrics.as_mut_ptr(),
            )?;

            Ok(HitTestPoint {
                is_trailing_hit: is_trailing_hit.as_bool(),
                metrics: HitTestMetrics::from_dwrite(&metrics.assume_init(), &self.text),
            })
        }
    }

    /// Returns the layout maximum size.
    pub fn max_size(&self) -> Size {
        unsafe {
            let w = self.layout.GetMaxWidth();
            let h = self.layout.GetMaxHeight();
            Size::new(w as f64, h as f64)
        }
    }

    pub fn hit_test_text_position(&self, text_position: usize) -> Result<HitTestTextPosition, Error> {
        // convert the text position to an utf-16 offset (inspired by piet-direct2d).
        let pos_utf16 = count_utf16(&self.text[0..text_position]);

        unsafe {
            let mut point_x = 0.0f32;
            let mut point_y = 0.0f32;
            let mut metrics = MaybeUninit::<DWRITE_HIT_TEST_METRICS>::uninit();
            self.layout.HitTestTextPosition(
                pos_utf16 as u32,
                false,
                &mut point_x,
                &mut point_y,
                metrics.as_mut_ptr(),
            )?;

            Ok(HitTestTextPosition {
                metrics: HitTestMetrics::from_dwrite(&metrics.assume_init(), &self.text),
                point: Point::new(point_x as f64, point_y as f64),
            })
        }
    }

    pub fn hit_test_text_range(&self, text_range: Range<usize>, origin: &Point) -> Result<Vec<HitTestMetrics>, Error> {
        unsafe {
            // convert range to UTF16
            let text_position = count_utf16(&self.text[0..text_range.start]);
            let text_length = count_utf16(&self.text[text_range]);

            // first call to determine the count
            let text_metrics = self.layout.GetMetrics()?;

            // "A good value to use as an initial value for maxHitTestMetricsCount
            // may be calculated from the following equation:
            // maxHitTestMetricsCount = lineCount * maxBidiReorderingDepth"
            // (https://docs.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittesttextrange)
            let mut max_metrics_count = text_metrics.lineCount * text_metrics.maxBidiReorderingDepth;
            let mut actual_metrics_count = 0;
            let mut metrics = Vec::with_capacity(max_metrics_count as usize);

            let result = self.layout.HitTestTextRange(
                text_position as u32,
                text_length as u32,
                origin.x as f32,
                origin.y as f32,
                metrics.as_mut_ptr(),
                max_metrics_count,
                &mut actual_metrics_count,
            );

            if let Err(e) = result {
                if e.code() == ERROR_INSUFFICIENT_BUFFER.into() {
                    // reallocate with sufficient space
                    metrics = Vec::with_capacity(actual_metrics_count as usize);
                    max_metrics_count = actual_metrics_count;
                    self.layout.HitTestTextRange(
                        text_position as u32,
                        text_length as u32,
                        origin.x as f32,
                        origin.y as f32,
                        metrics.as_mut_ptr(),
                        max_metrics_count,
                        &mut actual_metrics_count,
                    )?;
                } else {
                    return Err(e.into());
                }
            }

            metrics.set_len(actual_metrics_count as usize);
            Ok(metrics
                .into_iter()
                .map(|m| HitTestMetrics::from_dwrite(&m, &self.text))
                .collect())
        }
    }

    pub fn metrics(&self) -> TextMetrics {
        unsafe {
            let metrics = self.layout.GetMetrics().expect("GetMetrics failed");
            metrics.into()
        }
    }

    pub fn line_metrics(&self) -> Vec<LineMetrics> {
        unsafe {
            let mut line_count = 1;
            let mut metrics = Vec::with_capacity(line_count as usize);
            let result = self
                .layout
                .GetLineMetrics(metrics.as_mut_ptr(), line_count, &mut line_count);

            if let Err(e) = result {
                if e.code() == ERROR_INSUFFICIENT_BUFFER.into() {
                    // reallocate with sufficient space
                    metrics = Vec::with_capacity(line_count as usize);
                    self.layout
                        .GetLineMetrics(metrics.as_mut_ptr(), line_count, &mut line_count)
                        .expect("GetLineMetrics failed");
                }
            }

            metrics.set_len(line_count as usize);
            metrics.into_iter().map(|m| m.into()).collect()
        }
    }

    pub fn get_rasterized_glyph_runs(&self, scale_factor: f64, origin: Point) -> Vec<GlyphRun> {
        let transform = Transform::identity();

        let mut output_glyph_runs = Vec::new();

        let renderer: IDWriteTextRenderer = DWriteRendererProxy {
            scale_factor,
            transform,
            output_glyph_runs: &mut output_glyph_runs,
        }
        .into();

        unsafe {
            self.layout
                .Draw(ptr::null(), renderer, origin.x as f32, origin.y as f32)
        };

        output_glyph_runs
    }
}

#[derive(Clone, Debug)]
pub struct FontFace {
    font_face: IDWriteFontFace,
}

#[derive(Copy, Clone, Debug)]
pub struct GlyphOffset {
    advance_offset: f32,
    ascender_offset: f32,
}

#[derive(Debug)]
pub struct GlyphRun {
    alpha_texture: Vec<u8>,
    bounds: RectI,
}

/// Trait for rendering a series of glyph runs
pub trait Renderer {
    /// Draw a glyph run
    fn draw_glyph_run(&mut self, glyph_run: &GlyphRun);

    /// Returns the current text transformation.
    fn transform(&self) -> Transform<UnknownUnit, UnknownUnit>;
}

#[implement(IDWriteTextRenderer)]
struct DWriteRendererProxy {
    scale_factor: f64,
    transform: Transform<UnknownUnit, UnknownUnit>,
    output_glyph_runs: *mut Vec<GlyphRun>,
}

impl IDWritePixelSnapping_Impl for DWriteRendererProxy {
    fn IsPixelSnappingDisabled(&self, clientdrawingcontext: *const c_void) -> ::windows::core::Result<BOOL> {
        Ok(false.into())
    }

    fn GetCurrentTransform(&self, clientdrawingcontext: *const c_void) -> ::windows::core::Result<DWRITE_MATRIX> {
        let transform = DWRITE_MATRIX {
            m11: self.transform.m11 as f32,
            m12: self.transform.m12 as f32,
            m21: self.transform.m21 as f32,
            m22: self.transform.m22 as f32,
            dx: self.transform.m31 as f32,
            dy: self.transform.m32 as f32,
        };
        Ok(transform)
    }

    fn GetPixelsPerDip(&self, clientdrawingcontext: *const c_void) -> ::windows::core::Result<f32> {
        Ok(self.scale_factor as f32)
    }
}

impl IDWriteTextRenderer_Impl for DWriteRendererProxy {
    fn DrawGlyphRun(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        measuringmode: DWRITE_MEASURING_MODE,
        glyphrun: *const DWRITE_GLYPH_RUN,
        glyphrundescription: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        clientdrawingeffect: &Option<IUnknown>,
    ) -> ::windows::core::Result<()> {
        unsafe {
            let transform = DWRITE_MATRIX {
                m11: self.transform.m11 as f32,
                m12: self.transform.m12 as f32,
                m21: self.transform.m21 as f32,
                m22: self.transform.m22 as f32,
                dx: self.transform.m31 as f32,
                dy: self.transform.m32 as f32,
            };

            let glyph_run_analysis: IDWriteGlyphRunAnalysis = dwrite_factory().CreateGlyphRunAnalysis(
                glyphrun,
                self.scale_factor as f32,
                &transform,
                DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL,
                measuringmode,
                baselineoriginx,
                baselineoriginy,
            )?;

            let bounds: RECT = glyph_run_analysis.GetAlphaTextureBounds(DWRITE_TEXTURE_CLEARTYPE_3x1)?;
            let width = bounds.right - bounds.left;
            let height = bounds.bottom - bounds.top;
            let rendering_params = dwrite_factory()
                .CreateRenderingParams()
                .expect("CreateRenderingParams failed");

            let mut blend_gamma = 0.0f32;
            let mut blend_enhanced_contrast = 0.0f32;
            let mut blend_clear_type_level = 0.0f32;
            glyph_run_analysis.GetAlphaBlendParams(
                rendering_params,
                &mut blend_gamma,
                &mut blend_enhanced_contrast,
                &mut blend_clear_type_level,
            )?;

            let buffer_size = (3 * width * height) as usize;
            let mut alpha_texture = Vec::with_capacity(buffer_size);
            glyph_run_analysis.CreateAlphaTexture(
                DWRITE_TEXTURE_CLEARTYPE_3x1,
                &bounds,
                alpha_texture.as_mut_ptr(),
                buffer_size as u32,
            )?;
            alpha_texture.set_len(buffer_size);

            (&mut *self.output_glyph_runs).push(GlyphRun {
                alpha_texture,
                bounds: RectI::new(
                    PointI::new(bounds.left, bounds.top),
                    SizeI::new(bounds.right - bounds.left, bounds.bottom - bounds.top),
                ),
            });

            Ok(())
        }
    }

    fn DrawUnderline(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        underline: *const DWRITE_UNDERLINE,
        clientdrawingeffect: &Option<::windows::core::IUnknown>,
    ) -> ::windows::core::Result<()> {
        todo!()
    }

    fn DrawStrikethrough(
        &self,
        clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        strikethrough: *const DWRITE_STRIKETHROUGH,
        clientdrawingeffect: &Option<::windows::core::IUnknown>,
    ) -> ::windows::core::Result<()> {
        todo!()
    }

    fn DrawInlineObject(
        &self,
        clientdrawingcontext: *const c_void,
        originx: f32,
        originy: f32,
        inlineobject: &Option<IDWriteInlineObject>,
        issideways: BOOL,
        isrighttoleft: BOOL,
        clientdrawingeffect: &Option<::windows::core::IUnknown>,
    ) -> ::windows::core::Result<()> {
        todo!()
    }
}

/*#[implement(ID2D1SimplifiedGeometrySink)]
struct GeometrySink {}

impl GeometrySink {
    pub unsafe fn SetFillMode(&self, fillmode: D2D1_FILL_MODE) {
        todo!()
    }

    pub unsafe fn SetSegmentFlags(&self, vertexflags: D2D1_PATH_SEGMENT) {
        todo!()
    }

    pub unsafe fn BeginFigure(&self, startpoint: D2D_POINT_2F, figurebegin: D2D1_FIGURE_BEGIN) {
        todo!()
    }

    pub unsafe fn AddLines(&self, points: *const D2D_POINT_2F, pointscount: u32) {
        todo!()
    }

    pub unsafe fn AddBeziers(&self, beziers: *const D2D1_BEZIER_SEGMENT, bezierscount: u32) {
        todo!()
    }

    pub unsafe fn EndFigure(&self, figureend: D2D1_FIGURE_END) {
        todo!()
    }

    pub unsafe fn Close(&self) -> ::windows::core::Result<()> {
        todo!()
    }
}*/

impl FormattedText {
    pub fn create_paragraph(&self, layout_box_size: Size) -> Paragraph {
        unsafe {
            let text_wide = self.plain_text.to_wstring();

            // TODO locale name?

            // default format
            let default_font_family = "Arial".to_wstring();
            let locale_name = "".to_wstring();
            let format = dwrite_factory()
                .CreateTextFormat(
                    PCWSTR(default_font_family.as_ptr()),
                    None,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    14.0,
                    PCWSTR(locale_name.as_ptr()),
                )
                .expect("CreateTextFormat failed");

            let layout: IDWriteTextLayout = dwrite_factory()
                .CreateTextLayout(
                    PCWSTR(text_wide.as_ptr()),
                    text_wide.len() as u32,
                    format,
                    layout_box_size.width as f32,
                    layout_box_size.height as f32,
                )
                .expect("CreateTextLayout failed");

            // apply style ranges
            for run in self.runs.runs.iter() {
                let mut font_family = None;
                let mut font_weight = None;
                let mut font_style = None;
                //let mut font_stretch = None;
                let mut font_size = None;
                let mut color = None;

                for attr in run.attributes.iter() {
                    match *attr {
                        Attribute::FontSize(size) => font_size = Some(size),
                        Attribute::FontFamily(ref ff) => {
                            font_family = Some(ff);
                        }
                        Attribute::FontStyle(fs) => {
                            font_style = Some(fs);
                        }
                        Attribute::FontWeight(fw) => {
                            font_weight = Some(fw);
                        }
                        Attribute::Color(c) => {
                            color = Some(c);
                        }
                    }
                }

                let range = to_dwrite_text_range(&self.plain_text, run.range.clone());

                if let Some(ff) = font_family {
                    let ff_name = ff.0.to_wstring();
                    layout.SetFontFamilyName(PCWSTR(ff_name.as_ptr()), range);
                }

                if let Some(fs) = font_size {
                    layout.SetFontSize(fs as f32, range);
                }

                if let Some(fw) = font_weight {
                    layout.SetFontWeight(fw.to_dwrite(), range);
                }

                if let Some(fs) = font_style {
                    layout.SetFontStyle(fs.to_dwrite(), range);
                }
            }

            Paragraph {
                layout,
                text: self.plain_text.clone(),
            }
        }
    }
}
