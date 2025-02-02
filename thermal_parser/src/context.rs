use crate::decoder::{get_codepage, Codepage};
use crate::graphics;
use crate::graphics::{GraphicsCommand, ImageRef, RGBA};
use crate::text::TextSpan;
use std::collections::HashMap;
use std::mem;

#[derive(Clone, PartialEq, Debug)]
pub enum TextJustify {
    Left,
    Center,
    Right,
}

#[derive(Clone, PartialEq)]
pub enum TextStrikethrough {
    Off,
    On,
    Double,
}

#[derive(Clone, PartialEq)]
pub enum TextUnderline {
    Off,
    On,
    Double,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Font {
    A,
    B,
    C,
    D,
    E,
    SpecialA,
    SpecialB,
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
            _ => Font::A,
        }
    }
    //Currently the rest of the fonts default to font b
    //We don't have enough information on C D E or the special fonts
    pub fn to_size(&self) -> (u8, u8) {
        if self == &Font::A {
            (12, 24)
        } else {
            (9, 17)
        }
    }
}

#[derive(Clone, Debug)]
pub enum HumanReadableInterface {
    None,
    Above,
    Below,
    Both,
}

#[derive(Clone)]
pub struct Context {
    pub default: Option<Box<Context>>,
    pub text: TextContext,
    pub barcode: BarcodeContext,
    pub code2d: Code2DContext,
    pub graphics: GraphicsContext,
    pub page_mode: PageModeContext,
}

#[derive(Clone)]
pub struct TextContext {
    pub character_width: u8,
    pub character_height: u8,
    pub character_set: u8,
    pub code_table: u8,
    pub decoder: Codepage,
    pub font_size: u8,
    pub justify: TextJustify,
    pub font: Font,
    pub bold: bool,
    pub italic: bool,
    pub underline: TextUnderline,
    pub strikethrough: TextStrikethrough,
    pub invert: bool,
    pub width_mult: u8,
    pub height_mult: u8,
    pub upside_down: bool,
    pub line_spacing: u8,
    pub color: RGBA,
    pub background_color: RGBA,
    pub shadow_color: RGBA,
    pub shadow: bool,
    pub smoothing: bool,
    pub tabs: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct RenderColors {
    pub paper_color: RGBA,
    pub color_1: RGBA,
    pub color_2: RGBA,
    pub color_3: RGBA,
}

impl RenderColors {
    pub fn color_for_number(&self, number: u8) -> &RGBA {
        match number {
            0 => &self.paper_color,
            1 | 49 => &self.color_1,
            2 | 50 => &self.color_2,
            3 | 51 => &self.color_3,
            _ => &self.color_1,
        }
    }
}

#[derive(Clone)]
pub struct GraphicsContext {
    //Main rendering area
    pub render_area: RenderArea,

    pub render_colors: RenderColors,

    //Paper area (unprintable paper margins)
    //x and y represent left and right margins
    pub paper_area: RenderArea,

    pub dots_per_inch: u16,
    pub v_motion_unit: u8,
    pub h_motion_unit: u8,
    pub graphics_count: u16,
    pub stored_graphics: HashMap<ImageRef, GraphicsCommand>,
    pub buffer_graphics: Vec<GraphicsCommand>,
}

#[derive(Clone)]
pub struct BarcodeContext {
    pub human_readable: HumanReadableInterface,
    pub width: u8,
    pub height: u8,
    pub font: Font,
}

#[derive(Clone, Debug)]
pub enum QrModel {
    Model1, //Numeric data
    Model2, //Aplhanumeric data
    Micro,  //32 chars
}

#[derive(Clone, Debug)]
pub enum QrErrorCorrection {
    L,
    M,
    Q,
    H,
}

#[derive(Clone)]
pub struct Code2DContext {
    pub symbol_storage: Option<graphics::Code2D>,

    pub qr_model: QrModel,
    pub qr_error_correction: QrErrorCorrection,
    pub qr_size: u8,

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

#[derive(Clone, Debug)]
pub enum PrintDirection {
    TopLeft2Right,
    BottomRight2Left,
    TopRight2Bottom,
    BottomLeft2Top,
}

#[derive(Clone, Debug)]
pub struct RenderArea {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone)]
pub struct PageModeContext {
    //Is page mode enabled
    pub enabled: bool,

    //Raw renderable area
    pub logical_area: RenderArea,

    //Actual graphics context renderable area
    //Generally a translated version of the logical
    //area
    pub render_area: RenderArea,

    //Total page area, can grow when render area
    //is changed
    pub page_area: RenderArea,

    //Page mode print direction
    pub direction: PrintDirection,
    pub previous_direction: PrintDirection,
}

#[derive(Debug)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

impl PageModeContext {
    pub fn apply_logical_area(&mut self) -> (Rotation, u32, u32) {
        let rotation =
            self.calculate_directional_rotation(&self.previous_direction, &self.direction);

        //Swap page area w and h
        let previously_swapped = PageModeContext::should_dimension_swap(&self.previous_direction);
        let should_swap = PageModeContext::should_dimension_swap(&self.direction);

        //Swap page dimension
        if !previously_swapped && should_swap || !should_swap && previously_swapped {
            mem::swap(&mut self.page_area.w, &mut self.page_area.h);
        }

        //Translate logical area to render area
        match self.direction {
            PrintDirection::TopLeft2Right => self.translate_top_left_to_right(),
            PrintDirection::BottomRight2Left => self.translate_bottom_right_to_left(),
            PrintDirection::TopRight2Bottom => self.translate_top_right_to_bottom(),
            PrintDirection::BottomLeft2Top => self.translate_bottom_left_to_top(),
        };

        //Set base values for x and y, render area will use these when resetting to y=0
        self.page_area.x = self.render_area.x;
        self.page_area.y = self.render_area.y;

        let render_max_width = self.render_area.x + self.render_area.w;
        let render_max_height = self.render_area.y + self.render_area.h;

        self.page_area.w = render_max_width.max(self.page_area.w);
        self.page_area.h = render_max_height.max(self.page_area.h);

        (rotation, self.page_area.w, self.page_area.h)
    }

    pub fn set_x(&mut self, x: u32) {
        let r = &mut self.render_area;
        let p = &mut self.page_area;
        r.x = p.x + x;
    }

    pub fn set_y(&mut self, y: u32) {
        let r = &mut self.render_area;
        let p = &mut self.page_area;
        r.y = p.y + y;
    }

    //Absolute x and y are always from the 0,0 top left position
    pub fn set_x_absolute(&mut self, x: u32) {
        let r = &mut self.render_area;
        let p = &mut self.page_area;
        match self.direction {
            PrintDirection::TopLeft2Right => r.x = p.x + x, //g
            PrintDirection::BottomRight2Left => r.x = (p.x + r.w).saturating_sub(x), //g
            PrintDirection::TopRight2Bottom => r.y = r.h.saturating_sub(x), //?
            PrintDirection::BottomLeft2Top => r.y = p.y + x, //g
        }
    }

    //Absolute x and y are always from the 0,0 top left position
    pub fn set_y_absolute(&mut self, y: u32) {
        let r = &mut self.render_area;
        let p = &mut self.page_area;
        match self.direction {
            PrintDirection::TopLeft2Right => r.y = p.y + y, //g
            PrintDirection::BottomRight2Left => r.y = p.y + r.h.saturating_sub(y), //g
            PrintDirection::TopRight2Bottom => r.x = p.x + y, //g
            PrintDirection::BottomLeft2Top => r.x = p.x + r.w.saturating_sub(y), //g
        }
    }

    pub fn offset_x(&mut self, x: u32) {
        self.render_area.x += x;
    }

    pub fn offset_y(&mut self, y: u32) {
        self.render_area.y += y;
    }

    pub fn offset_x_relative(&mut self, x: i16) {
        let mut new_x = self.render_area.x as i32 + x as i32;
        if new_x < 0 {
            new_x = 0;
        }
        self.render_area.x = new_x as u32;
    }

    pub fn offset_y_relative(&mut self, y: i16) {
        let mut new_y = self.render_area.y as i32 + y as i32;
        if new_y < 0 {
            new_y = 0;
        }
        self.render_area.y = new_y as u32;
    }

    fn should_dimension_swap(direction: &PrintDirection) -> bool {
        match direction {
            PrintDirection::TopLeft2Right | PrintDirection::BottomRight2Left => false,
            _ => true,
        }
    }

    fn translate_top_left_to_right(&mut self) {
        let l = &self.logical_area;
        let r = &mut self.render_area;

        r.w = l.w;
        r.h = l.h;
        r.x = l.x;
        r.y = l.y;
    }

    fn translate_bottom_right_to_left(&mut self) {
        let l = &self.logical_area;
        let r = &mut self.render_area;
        let p = &mut self.page_area;

        r.w = l.w;
        r.h = l.h;
        r.y = l.y;
        r.x = p.w.saturating_sub(l.x + l.w);
    }

    fn translate_top_right_to_bottom(&mut self) {
        let l = &self.logical_area;
        let r = &mut self.render_area;
        let p = &mut self.page_area;

        r.w = l.h;
        r.h = l.w;
        r.x = p.w.saturating_sub(l.y + l.h);
        r.y = p.h.saturating_sub(l.x + l.w);
    }

    fn translate_bottom_left_to_top(&mut self) {
        let l = &self.logical_area;
        let r = &mut self.render_area;
        let p = &mut self.page_area;

        r.w = l.h;
        r.h = l.w;
        r.x = p.w.saturating_sub(l.y + l.h);
        r.y = l.x;
    }

    pub fn calculate_directional_rotation(
        &self,
        from: &PrintDirection,
        to: &PrintDirection,
    ) -> Rotation {
        let previous = match from {
            PrintDirection::TopRight2Bottom => 3,
            PrintDirection::BottomRight2Left => 2,
            PrintDirection::BottomLeft2Top => 1,
            PrintDirection::TopLeft2Right => 0,
        };

        let current = match to {
            PrintDirection::TopRight2Bottom => 3,
            PrintDirection::BottomRight2Left => 2,
            PrintDirection::BottomLeft2Top => 1,
            PrintDirection::TopLeft2Right => 0,
        };

        let orientation_delta = (current as i8 - previous as i8).rem_euclid(4) as u8;

        //Come up with the rotation change that will
        //put page mode render area into the correct
        //render orientation
        match orientation_delta {
            1 => Rotation::R90,
            2 => Rotation::R180,
            3 => Rotation::R270,
            _ => Rotation::R0,
        }
    }
}

impl Context {
    fn default() -> Context {
        let dots_per_inch = 203;
        let paper_left_margin = (dots_per_inch as f32 * 0.1f32) as u32;
        let paper_right_margin = (dots_per_inch as f32 * 0.1f32) as u32;
        let paper_width = (dots_per_inch as f32 * 3.2f32) as u32;
        let render_width = paper_width - (paper_left_margin + paper_right_margin);
        let render_colors = RenderColors {
            paper_color: RGBA {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }, //White
            color_1: RGBA {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }, //Black
            color_2: RGBA {
                r: 158,
                g: 22,
                b: 22,
                a: 255,
            }, //Red
            color_3: RGBA {
                r: 27,
                g: 57,
                b: 169,
                a: 255,
            }, //Blue
        };

        Context {
            default: None,
            text: TextContext {
                character_width: 12,
                character_height: 24,
                character_set: 0,
                code_table: 0,
                decoder: get_codepage(0, 0),
                font_size: 10,
                justify: TextJustify::Left,
                font: Font::A,
                bold: false,
                italic: false,
                underline: TextUnderline::Off,
                strikethrough: TextStrikethrough::Off,
                invert: false,
                width_mult: 1,
                height_mult: 1,
                upside_down: false,
                line_spacing: 24, //pixels
                color: render_colors.color_1,
                background_color: render_colors.paper_color,
                shadow: false,
                shadow_color: render_colors.color_1,
                smoothing: false,
                tabs: vec![8; 32], //Every 8 character widths is a tab stop
            },
            barcode: BarcodeContext {
                human_readable: HumanReadableInterface::None,
                width: 3,
                height: 40,
                font: Font::A,
            },
            code2d: Code2DContext {
                symbol_storage: None,
                qr_model: QrModel::Model1,
                qr_error_correction: QrErrorCorrection::L,
                qr_size: 3,
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
                render_colors,

                render_area: RenderArea {
                    x: 0,
                    y: paper_left_margin * 3,
                    w: render_width,
                    h: 0,
                },
                paper_area: RenderArea {
                    x: paper_left_margin,
                    y: paper_right_margin,
                    w: paper_width,
                    h: 0,
                },
                dots_per_inch,
                //Both of these motion units are used for
                //Various positioning commands in standard mode
                //and in page mode.
                v_motion_unit: 1, //Pixels per unit
                h_motion_unit: 1, //Pixels per unit
                graphics_count: 0,
                stored_graphics: HashMap::<ImageRef, GraphicsCommand>::new(),
                buffer_graphics: vec![],
            },
            page_mode: PageModeContext {
                enabled: false,
                logical_area: RenderArea {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                render_area: RenderArea {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                page_area: RenderArea {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                direction: PrintDirection::TopLeft2Right,
                previous_direction: PrintDirection::TopLeft2Right,
            },
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

    pub fn font_size_pixels(&self) -> u32 {
        //1 point = 72 pixels
        let pixels_per_point = self.graphics.dots_per_inch as f32 / 96f32;
        (self.text.font_size as f32 * pixels_per_point) as u32
    }

    pub fn points_to_pixels(&self, points: f32) -> u32 {
        let pixels_per_point = self.graphics.dots_per_inch as f32 / 96f32;
        (points * pixels_per_point) as u32
    }

    pub fn set_tab_len(&mut self, tab_count: u8, at: u8) {
        if at < self.text.tabs.len() as u8 {
            self.text.tabs[at as usize] = tab_count;
        }
    }

    //Reset the x to the base value
    //which is the furthest left
    pub fn reset_x(&mut self) {
        if self.page_mode.enabled {
            self.page_mode.render_area.x = self.get_base_x();
        } else {
            self.graphics.render_area.x = self.get_base_x();
        }
    }

    pub fn reset_y(&mut self) {
        if self.page_mode.enabled {
            self.page_mode.render_area.y = self.get_base_y();
        } else {
            self.graphics.render_area.y = self.get_base_y();
        }
    }

    //The base x value, which is the furthest left
    //of the render area
    pub fn get_base_x(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.page_area.x
        } else {
            0
        }
    }

    pub fn get_base_y(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.page_area.y
        } else {
            0
        }
    }

    pub fn get_x(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.render_area.x
        } else {
            self.graphics.render_area.x
        }
    }

    pub fn get_y(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.render_area.y
        } else {
            self.graphics.render_area.y
        }
    }

    pub fn offset_x(&mut self, x: u32) {
        if self.page_mode.enabled {
            self.page_mode.offset_x(x);
        } else {
            self.graphics.render_area.x += x;
        }
    }

    pub fn offset_y(&mut self, y: u32) {
        if self.page_mode.enabled {
            self.page_mode.offset_y(y);
        } else {
            self.graphics.render_area.y += y;
        }
    }

    //Uses motion units
    pub fn offset_x_relative(&mut self, x: i16) {
        let adj_x = x.saturating_div(self.graphics.h_motion_unit as i16);

        if self.page_mode.enabled {
            self.page_mode.offset_x_relative(adj_x);
        } else {
            let mut new_x = self.graphics.render_area.x as i32 + adj_x as i32;
            if new_x < 0 {
                new_x = 0;
            }
            self.graphics.render_area.x = new_x as u32;
        }
    }

    //Uses motion units
    pub fn offset_y_relative(&mut self, y: i16) {
        let adj_y = y.saturating_div(self.graphics.v_motion_unit as i16);

        if self.page_mode.enabled {
            self.page_mode.offset_y_relative(adj_y);
        } else {
            let mut new_y = self.graphics.render_area.y as i32 + adj_y as i32;
            if new_y < 0 {
                new_y = 0;
            }
            self.graphics.render_area.y = new_y as u32;
        }
    }

    pub fn feed(&mut self, motion_units: u32) {
        self.offset_y(motion_units);
        self.reset_x();
    }

    pub fn newline(&mut self, count: u32) {
        let line_height = self.text.line_spacing as u32;
        self.reset_x();
        self.offset_y(line_height * count);
    }

    pub fn newline_for_spans(&mut self, spans: &Vec<TextSpan>) {
        let mut line_height = self.text.line_spacing as u32;

        for span in spans {
            line_height = line_height.max(span.character_height);
        }

        self.reset_x();
        self.offset_y(line_height);
    }

    pub fn set_font(&mut self, font: Font) {
        let size = font.to_size();
        self.text.font = font;
        self.text.character_width = size.0;
        self.text.character_height = size.1;
    }

    pub fn set_x(&mut self, x: u32) {
        if self.page_mode.enabled {
            self.page_mode.set_x(x);
        } else {
            self.graphics.render_area.x = x;
        }
    }

    pub fn set_y(&mut self, y: u32) {
        if self.page_mode.enabled {
            self.page_mode.set_y(y);
        } else {
            self.graphics.render_area.y = y;
        }
    }

    //Uses motion units
    pub fn set_x_absolute(&mut self, x: u32) {
        let adj_x = x.saturating_div(self.graphics.h_motion_unit as u32);
        if self.page_mode.enabled {
            self.page_mode.set_x_absolute(adj_x);
        } else {
            self.graphics.render_area.x = adj_x;
        }
    }

    //Uses motion units
    pub fn set_y_absolute(&mut self, y: u32) {
        let adj_y = y.saturating_div(self.graphics.v_motion_unit as u32);
        if self.page_mode.enabled {
            self.page_mode.set_y_absolute(adj_y);
        } else {
            self.graphics.render_area.y = adj_y;
        }
    }

    pub fn set_page_area(&mut self, area: RenderArea) {
        let mut adj_area = area.clone();

        //Area needs to be adjusted based on motion units
        adj_area.x = adj_area
            .x
            .saturating_div(self.graphics.h_motion_unit as u32);
        adj_area.y = adj_area
            .y
            .saturating_div(self.graphics.v_motion_unit as u32);
        adj_area.w = adj_area
            .w
            .saturating_div(self.graphics.h_motion_unit as u32);
        adj_area.h = adj_area
            .h
            .saturating_div(self.graphics.v_motion_unit as u32);

        self.page_mode.logical_area = adj_area;
    }

    pub fn get_width(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.render_area.w
        } else {
            self.graphics.render_area.w
        }
    }

    pub fn get_available_width(&self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.render_area.w.saturating_sub(
                self.page_mode
                    .render_area
                    .x
                    .saturating_sub(self.page_mode.page_area.x),
            )
        } else {
            if self.graphics.render_area.x <= self.graphics.render_area.w {
                self.graphics
                    .render_area
                    .w
                    .saturating_sub(self.graphics.render_area.x)
            } else {
                0
            }
        }
    }

    pub fn get_height(&mut self) -> u32 {
        if self.page_mode.enabled {
            self.page_mode.render_area.h
        } else {
            self.graphics.render_area.h
        }
    }

    pub fn calculate_justification(&self, width: u32) -> u32 {
        let w = width;
        let render_width = if self.page_mode.enabled {
            self.page_mode.render_area.w
        } else {
            self.graphics.render_area.w
        };

        if w > render_width {
            return 0;
        }
        match self.text.justify {
            TextJustify::Center => {
                let center_remaining = render_width - w;
                if center_remaining > 0 {
                    (center_remaining / 2) as u32
                } else {
                    0
                }
            }
            TextJustify::Right => render_width - w,
            _ => 0,
        }
    }

    pub fn line_height_pixels(&self) -> u32 {
        self.text.line_spacing as u32
    }

    pub fn update_decoder(&mut self) {
        self.text.decoder = get_codepage(self.text.code_table, self.text.character_set);

        //Codepage 255 is used specifically in this project for UTF8 encoded text
        self.text.decoder.use_utf8_table = if self.text.code_table == 255 {
            true
        } else {
            false
        };
    }
}
