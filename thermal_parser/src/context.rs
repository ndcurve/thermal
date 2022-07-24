use std::collections::HashMap;

use crate::graphics::{Code2D, Image, ImageRef};

#[derive(Clone)]
pub enum TextJustify { Left, Center, Right }

#[derive(Clone, PartialEq)]
pub enum TextUnderline { Off, On, Double }

#[derive(Clone)]
pub enum Font {
    A, B, C, D, E, SpecialA, SpecialB,
}

impl Font {
    pub fn from_raw(byte: u8) -> Font {
        match byte {
            0 | 48 => Font::A,
            1 | 49 => Font::B,
            2 | 50 => Font::C,
            3 | 51 => Font::D,
            4 | 52 => Font::E,
            97 => Font::SpecialA,
            98 => Font::SpecialB,
            _ => Font::A
        }
    }
}

#[derive(Clone)]
pub enum HumanReadableInterface {
    None,
    Above,
    Below,
    Both,
}

#[derive(Clone)]
pub enum Color{
    Black,
    Red
}


#[derive(Clone)]
pub struct Context {
    pub default: Option<Box<Context>>,
    pub text: TextContext,
    pub barcode: BarcodeContext,
    pub code2d: Code2DContext,
    pub graphics: GraphicsContext,
}

#[derive(Clone)]
pub struct TextContext {
    pub character_set: u8,
    pub code_table: u8,
    pub font_size: u8,
    pub justify: TextJustify,
    pub font: Font,
    pub bold: bool,
    pub italic: bool,
    pub underline: TextUnderline,
    pub invert: bool,
    pub width_mult: u8,
    pub height_mult: u8,
    pub upside_down: bool,
    pub line_spacing: u8,
    pub color: Color,
    pub smoothing: bool
}

#[derive(Clone)]
pub struct GraphicsContext {
    pub dots_per_inch: u16,
    pub v_motion_unit: f32,
    pub h_motion_unit: f32,
    pub graphics_count: u16,
    pub stored_graphics: HashMap<ImageRef, Image>,
    pub buffer_graphics: Option<Image>,
}

#[derive(Clone)]
pub struct BarcodeContext {
    pub human_readable: HumanReadableInterface,
    pub width: u8,
    pub height: u8,
    pub font: Font,
}

#[derive(Clone)]
pub struct Code2DContext {
    pub symbol_storage: Option<Code2D>,

    pub qr_model: u8,
    pub qr_size: u8,
    pub qr_err_correction: u8,

    pub pdf417_columns: u8,
    pub pdf417_rows: u8,
    pub pdf417_width: u8,
    pub pdf417_row_height: u8,
    pub pdf417_err_correction: u8,
    pub pdf417_is_truncated: bool,

    pub maxicode_mode: u8,

    pub gs1_databar_width: u8,
    pub gs1_databar_max_width: u32,

    pub composite_width: u8,
    pub composite_max_width: u32,
    pub composite_font: Font,

    pub aztec_mode: u8,
    pub aztec_layers: u8,
    pub aztec_size: u8,
    pub aztec_error_correction: u8,

    pub datamatrix_type: u8,
    pub datamatrix_columns: u8,
    pub datamatrix_rows: u8,
    pub datamatrix_width: u8,
}

impl Context {
    fn default() -> Context {
        Context{
            default: None,
            text: TextContext {
                character_set: 0,
                code_table: 0,
                font_size: 16,
                justify: TextJustify::Left,
                font: Font::A,
                bold: false,
                italic: false,
                underline: TextUnderline::Off,
                invert: false,
                width_mult: 1,
                height_mult: 1,
                upside_down: false,
                line_spacing: 1,
                color: Color::Black,
                smoothing: false
            },
            barcode: BarcodeContext {
                human_readable: HumanReadableInterface::None,
                width: 2,
                height: 40,
                font: Font::A,
            },
            code2d: Code2DContext {
                symbol_storage: None,
                qr_model: 0,
                qr_size: 0,
                qr_err_correction: 0,
                pdf417_columns: 0,
                pdf417_rows: 0,
                pdf417_width: 0,
                pdf417_row_height: 0,
                pdf417_err_correction: 0,
                pdf417_is_truncated: false,
                maxicode_mode: 0,
                gs1_databar_width: 0,
                gs1_databar_max_width: 0,
                composite_width: 0,
                composite_max_width: 0,
                composite_font: Font::A,
                aztec_mode: 0,
                aztec_layers: 0,
                aztec_size: 0,
                aztec_error_correction: 0,
                datamatrix_type: 0,
                datamatrix_columns: 0,
                datamatrix_rows: 0,
                datamatrix_width: 0,
            },
            graphics: GraphicsContext {
                dots_per_inch: 180,
                v_motion_unit: 0.01,
                h_motion_unit: 0.01,
                graphics_count: 0,
                stored_graphics: HashMap::<ImageRef, Image>::new(),
                buffer_graphics: None
            }
        }
    }

    pub fn new() -> Context {
        let default_context = Context::default();
        let mut new_context = default_context.clone();
        new_context.default = Some(Box::from(default_context));
        new_context
    }

    pub fn reset(&mut self) {
        if let Some(default) = &self.default {
            self.text = default.text.clone();
            self.barcode = default.barcode.clone();
            self.code2d = default.code2d.clone();
            self.graphics = default.graphics.clone();
        }
    }
}