//! Thermal Renderer
//!
//! Creating your own renderer is made simple
//! by abstracting away all the finicky context
//! modifications.
//!
//! Renderer takes care of all positioning and then delegates the
//! actual rendering to the OutputRenderer.
//!
//! When you create an OutputRenderer, you will need to define
//! what kind of output the renderer creates.
//!
//! The ImageRenderer is a good place to look at for an example
//! of how to implement an OutputRenderer.
//!

use crate::renderer::RenderErrorKind::ChildRenderError;
use std::{fmt, mem};
use thermal_parser::command::{Command, CommandType, DeviceCommand};
use thermal_parser::context::{Context, HumanReadableInterface, Rotation, TextJustify};
use thermal_parser::graphics::{
    Barcode, Code2D, GraphicsCommand, Image, ImageFlow, Rectangle, VectorGraphic,
};
use thermal_parser::text::TextSpan;

#[derive(Debug, Clone, Copy)]
pub struct DebugProfile {
    pub text: bool,
    pub image: bool,
    pub page: bool,
    pub info: bool,
}

impl Default for DebugProfile {
    fn default() -> Self {
        DebugProfile {
            text: false,
            image: false,
            page: false,
            info: false,
        }
    }
}

pub struct RenderOutput<Output> {
    pub output: Vec<Output>,
    pub errors: Vec<RenderError>,
}

#[derive(Debug)]
pub enum RenderErrorKind {
    ChildRenderError,
    GraphicsError,
    UnknownCommand,
}

pub struct RenderError {
    kind: RenderErrorKind,
    description: String,
}

impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "❌ [{:?}] {}", self.kind, self.description)
    }
}

pub struct Renderer<'a, Output> {
    renderer: &'a mut Box<dyn OutputRenderer<Output>>,
    output_buffer: Vec<Output>,
    error_buffer: Vec<RenderError>,
    span_buffer: Vec<TextSpan>,
    context: Context,
    debug_profile: DebugProfile,
}

impl<'a, Output> Renderer<'a, Output> {
    pub fn new(
        renderer: &'a mut Box<(dyn OutputRenderer<Output> + 'static)>,
        debug_profile: DebugProfile,
    ) -> Self {
        Renderer {
            renderer,
            context: Context::new(),
            span_buffer: vec![],
            error_buffer: vec![],
            output_buffer: vec![],
            debug_profile,
        }
    }

    fn log_debug_icon(&self, icon: &str, description: &str) {
        if self.debug_profile.info {
            println!("├─ \x1b[0;36m{}\x1b[0m {}", icon, description);
        }
    }

    fn log_debug(&self, description: &str) {
        if self.debug_profile.info {
            println!("├─ {}", description);
        }
    }

    fn log_debug_start(&self, description: &str) {
        if self.debug_profile.info {
            println!("┌─ \x1b[0;32m→\x1b[0m {}", description);
        }
    }

    fn log_debug_end(&self, description: &str) {
        if self.debug_profile.info {
            println!("└─ {}", description);
        }
    }

    fn log_error(&mut self, kind: RenderErrorKind, description: String) {
        self.error_buffer.push(RenderError { kind, description });
    }

    pub fn render(&mut self, bytes: &Vec<u8>) -> RenderOutput<Output> {
        self.renderer.set_debug_profile(self.debug_profile);
        self.log_debug_start("Begin Render");

        let commands = thermal_parser::parse_esc_pos(bytes);

        for command in commands {
            self.log_debug(&format!(
                "{}",
                command.handler.debug(&command, &self.context)
            ));
            self.process_command(&command);
        }

        let mut output = vec![];
        let mut errors = vec![];

        mem::swap(&mut output, &mut self.output_buffer);
        mem::swap(&mut errors, &mut self.error_buffer);

        self.log_debug_end("End Render");

        RenderOutput { output, errors }
    }

    //default implementation
    fn process_command(&mut self, command: &Command) {
        match command.kind {
            CommandType::Unknown => {
                self.process_text();
                self.log_error(
                    RenderErrorKind::UnknownCommand,
                    command.handler.debug(command, &self.context),
                );
            }
            CommandType::Text => {
                let maybe_text = command.handler.get_text(command, &self.context);
                if let Some(text) = maybe_text {
                    self.collect_text(text);
                }
            }
            CommandType::Graphics => {
                self.process_text();

                let maybe_gfx = command.handler.get_graphics(command, &mut self.context);

                if let Some(gfx) = maybe_gfx {
                    match gfx {
                        GraphicsCommand::Error(error) => {
                            self.log_error(RenderErrorKind::GraphicsError, error);
                        }
                        GraphicsCommand::Code2D(code_2d) => {
                            self.process_code_2d(&code_2d);
                        }
                        GraphicsCommand::Barcode(barcode) => {
                            self.process_barcode(&barcode);
                        }
                        GraphicsCommand::Image(mut image) => {
                            self.process_image(&mut image);
                        }
                        GraphicsCommand::Rectangle(_) => {}
                        GraphicsCommand::Line(_) => {}
                    }
                }

                //Some graphics commands emit device commands
                let device_commands = &command
                    .handler
                    .get_device_command(command, &mut self.context);
                self.process_device_commands(device_commands);
            }
            CommandType::Context => {
                self.process_text();
                command.handler.apply_context(command, &mut self.context);
            }

            CommandType::ContextControl => {
                self.process_text();
                command.handler.apply_context(command, &mut self.context);

                let device_commands = &command
                    .handler
                    .get_device_command(command, &mut self.context);
                self.process_device_commands(device_commands);
            }
            CommandType::Control => {
                let device_commands = &command
                    .handler
                    .get_device_command(command, &mut self.context);
                self.process_text();
                self.process_device_commands(device_commands);
            }
            //This is a ContextControl but with the additional
            //fact that text is not collected
            CommandType::TextStyle => {
                command.handler.apply_context(command, &mut self.context);

                let device_commands = &command
                    .handler
                    .get_device_command(command, &mut self.context);
                self.process_device_commands(device_commands);
            }
            _ => {}
        }
    }

    fn process_device_commands(&mut self, device_commands: &Option<Vec<DeviceCommand>>) {
        if let Some(device_commands) = device_commands {
            for device_command in device_commands {
                self.renderer
                    .device_command(&mut self.context, device_command);

                match device_command {
                    DeviceCommand::SetTextWidth(w) => {
                        self.context.text.width_mult = *w;
                    }
                    DeviceCommand::SetTextHeight(h) => {
                        self.context.text.height_mult = *h;
                    }
                    DeviceCommand::Justify(j) => {
                        self.context.text.justify = j.clone();
                    }
                    DeviceCommand::BeginPrint => {
                        //Start the render at two newlines worth of height
                        self.context.newline(2);
                        self.renderer.begin_render(&mut self.context)
                    }
                    DeviceCommand::EndPrint => {
                        let errors = self.renderer.get_render_errors();

                        for error in errors {
                            self.log_error(ChildRenderError, error);
                        }

                        let output = self.renderer.end_render(&mut self.context);
                        self.output_buffer.push(output);
                    }
                    DeviceCommand::FeedLine(num_lines) => {
                        self.context.newline(*num_lines as u32);
                    }
                    DeviceCommand::Feed(num) => {
                        self.context.feed(*num as u32);
                    }
                    DeviceCommand::FullCut | DeviceCommand::PartialCut => {
                        self.context.newline(2);
                    }
                    DeviceCommand::BeginPageMode => {
                        self.context.page_mode.enabled = true;
                        self.renderer.page_begin(&mut self.context);
                    }
                    DeviceCommand::EndPageMode => {
                        self.renderer.page_end(&mut self.context);
                        self.context.page_mode.enabled = false
                    }
                    DeviceCommand::PrintPageMode => {
                        self.renderer.render_page(&mut self.context);

                        //Advance the y since a page is being rendered
                        self.context.graphics.render_area.y += self.context.page_mode.page_area.h;
                        self.context.graphics.render_area.x = 0;
                    }
                    DeviceCommand::ChangePageArea => {
                        //This is important to make sure that we know the direction has already been altered
                        self.context.page_mode.previous_direction =
                            self.context.page_mode.direction.clone();
                        let (rotation, width, height) = self.context.page_mode.apply_logical_area();
                        self.renderer
                            .page_area_changed(&mut self.context, rotation, width, height);
                    }
                    DeviceCommand::ChangePageModeDirection => {
                        let (rotation, width, height) = self.context.page_mode.apply_logical_area();
                        self.renderer
                            .page_area_changed(&mut self.context, rotation, width, height);
                    }
                    DeviceCommand::ChangeTabs(count, at) => {
                        self.context.set_tab_len(*count, *at);
                    }
                    DeviceCommand::ClearBufferGraphics => {
                        self.context.graphics.buffer_graphics.clear();
                    }
                    _ => {}
                }
            }
        }
    }

    fn process_code_2d(&mut self, code_2d: &Code2D) {
        let context = &mut self.context;
        let mut graphics = vec![];

        let mut i = 1;
        let origin_x = context.calculate_justification(code_2d.width as u32 * code_2d.point_width);
        context.set_x(origin_x);

        for p in &code_2d.points {
            if i != 1 && i % code_2d.width == 1 {
                context.set_x(origin_x);
                context.offset_y(code_2d.point_height as u32);
            }

            if *p > 0 {
                //Prevent rendering outside of print area
                if context.get_available_width() < code_2d.point_width as u32 {
                    continue;
                }

                graphics.push(VectorGraphic::Rectangle(Rectangle {
                    x: context.get_x(),
                    y: context.get_y(),
                    w: code_2d.point_width as u32,
                    h: code_2d.point_height as u32,
                }));
            }
            context.offset_x(code_2d.point_width as u32);
            i += 1;
        }

        context.reset_x();

        self.renderer.render_graphics(context, &graphics);
    }

    fn process_barcode(&mut self, barcode: &Barcode) {
        let mut graphics = vec![];

        match self.context.barcode.human_readable {
            HumanReadableInterface::Above | HumanReadableInterface::Both => {
                self.collect_text(barcode.text.clone());
                self.process_text();
                self.context.newline(1);
            }
            _ => {}
        }

        self.context.set_x(
            self.context
                .calculate_justification(barcode.points.len() as u32 * barcode.point_width as u32),
        );

        for bp in &barcode.points {
            if *bp > 0 {
                //Prevent rendering when beyond page bounds
                if self.context.get_available_width() < barcode.point_width as u32 {
                    continue;
                }

                graphics.push(VectorGraphic::Rectangle(Rectangle {
                    x: self.context.get_x(),
                    y: self.context.get_y(),
                    w: barcode.point_width as u32,
                    h: barcode.point_height as u32,
                }));
            }
            self.context.offset_x(barcode.point_width as u32);
        }

        self.log_debug_icon("║║", "Render Barcode");
        self.renderer.render_graphics(&mut self.context, &graphics);

        self.context.reset_x();
        self.context.offset_y(barcode.point_height as u32);

        match self.context.barcode.human_readable {
            HumanReadableInterface::Below | HumanReadableInterface::Both => {
                self.context.offset_y(8);
                self.collect_text(barcode.text.clone());
                self.process_text();
                self.context.newline(1);
            }
            _ => {}
        }
    }

    fn process_image(&mut self, image: &mut Image) {
        //let context = &mut self.context;

        match image.flow {
            ImageFlow::Inline => {
                if image.w > self.context.get_available_width() {
                    self.context.newline(1);
                }
            }
            ImageFlow::Block => {
                if !self.context.page_mode.enabled {
                    self.context
                        .set_x(self.context.calculate_justification(image.w));
                }
            }
            ImageFlow::None => {}
        }

        image.x = self.context.get_x();
        image.y = self.context.get_y();
        self.log_debug_icon("[§]", "Render Image");
        self.renderer.render_image(&mut self.context, image);

        match image.flow {
            ImageFlow::Inline => {
                self.context.offset_x(image.w);
            }
            ImageFlow::Block => {
                if !self.context.page_mode.enabled {
                    self.context.offset_y(image.h);
                    self.context.reset_x();
                }
            }
            _ => {}
        }
    }

    fn collect_text(&mut self, text: TextSpan) {
        self.span_buffer.push(text);
    }

    fn process_text(&mut self) {
        if self.span_buffer.is_empty() {
            return;
        }

        let mut words: Vec<TextSpan> = vec![];

        for span in &self.span_buffer {
            let mut spans: Vec<TextSpan> = span.break_into_words();
            words.append(&mut spans);
        }

        self.span_buffer.clear();

        let mut lines: Vec<Vec<TextSpan>> = vec![];
        let mut current_line: Vec<TextSpan> = vec![];
        let max_width = self.context.get_width();
        words.reverse();

        while let Some(mut word) = words.pop() {
            //Calculate available width every loop
            let avail_width = self.context.get_available_width();
            let word_width = word.get_width();

            //Newlines advance y and reset x
            if word.text.eq("\n") {
                //Advance line height
                self.context.newline_for_spans(&current_line);

                //Swap current line
                let mut finished_line = vec![];
                mem::swap(&mut current_line, &mut finished_line);
                lines.push(finished_line);

                //Start a new line
                lines.push(vec![]); //Newline
                continue;
            }

            //Tabs have a special behavior
            if word.text.eq("\t") {
                let current_x = self.context.get_x();
                let mut current_tab_pos = 0;
                for tab_len in &self.context.text.tabs {
                    if current_tab_pos >= current_x {
                        self.context.set_x(current_tab_pos);
                        break;
                    }
                    current_tab_pos += *tab_len as u32 * word.character_width;
                }
                continue;
            }

            if word_width <= avail_width {
                //Word fits into the line, add it
                word.get_dimensions(&self.context);
                self.context.offset_x(word.get_width());
                current_line.push(word);
                continue;
            } else if word_width > max_width {
                //Break the word into parts for super long words
                let mut broken = word.break_apart(
                    (avail_width / word.character_width) as usize,
                    (max_width / word.character_width).max(word.character_width) as usize,
                );

                let broken_len = broken.len() - 1;
                for (i, broke) in broken.iter_mut().enumerate() {
                    let last = broken_len == i;
                    broke.get_dimensions(&self.context);
                    current_line.push(broke.clone()); //ugg

                    if last {
                        //Last word doesn't geta a forced newline
                        self.context.offset_x(broke.get_width());
                    } else {
                        //Every other line we assume will fit into a line

                        //Advance line
                        self.context.newline_for_spans(&current_line);

                        //Swap line
                        let mut finished_line = vec![];
                        mem::swap(&mut current_line, &mut finished_line);
                        lines.push(finished_line);
                    }
                }
            } else {
                //Close out previous line
                let mut finished_line = vec![];
                self.context.newline_for_spans(&current_line);
                mem::swap(&mut current_line, &mut finished_line);
                lines.push(finished_line);

                //Add text to newline at 0 x
                let word_width = word.get_width();
                word.get_dimensions(&self.context);
                current_line.push(word);

                //Advance the x
                self.context.offset_x(word_width);
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        //Adjust lines for justification
        for line in &lines {
            if line.is_empty() {
                continue;
            }
            let justification = line.first().unwrap().justify.clone();

            let max_width = self.context.get_width();
            let mut max_height = 0;
            let mut line_width = 0;
            let mut line_offset = 0;

            for span in line {
                line_width += span.get_width();
                max_height = max_height.max(span.character_height);
            }

            match justification {
                TextJustify::Right => {
                    line_offset = max_width - line_width;
                }
                TextJustify::Center => {
                    if line_width < max_width {
                        line_offset = (max_width - line_width) / 2;
                    }
                }
                _ => {}
            }

            self.log_debug_icon(
                "🗚",
                &format!("Render Text {:?} at x offset = {}", line, line_offset),
            );

            self.renderer.render_text(
                &mut self.context,
                line,
                line_offset,
                max_height,
                justification,
            );
        }
    }
}

/// Implement the  Output Renderer in order to render to your own format.
///
/// The main Renderer takes care of all positioning of the xy coordinates.
///
/// You just need to render the elements at the provided xy and width height.
pub trait OutputRenderer<Output> {
    /// Possibly use the debug profile
    fn set_debug_profile(&mut self, profile: DebugProfile);

    /// Do setup steps here for each page output
    /// This can get called multiple times
    fn begin_render(&mut self, context: &mut Context);

    /// Page mode has started
    fn page_begin(&mut self, _context: &mut Context);
    fn page_area_changed(
        &mut self,
        _context: &mut Context,
        _rotation: Rotation,
        _width: u32,
        _height: u32,
    );

    /// Page mode has ended
    fn page_end(&mut self, _context: &mut Context) {}

    /// Render the page mode area to the main paper
    fn render_page(&mut self, _context: &mut Context);

    /// Render vector graphics
    fn render_graphics(&mut self, context: &mut Context, graphics: &Vec<VectorGraphic>);

    /// Render images
    fn render_image(&mut self, context: &mut Context, image: &Image);

    /// Render text
    fn render_text(
        &mut self,
        context: &mut Context,
        spans: &Vec<TextSpan>,
        x_offset: u32,
        max_height: u32,
        text_justify: TextJustify,
    );

    /// Possibly render or do something with a device command
    fn device_command(&mut self, _context: &mut Context, _command: &DeviceCommand) {}

    /// During rendering, if there are any errors that
    /// would fail a test, return them in this call
    /// Generally gets called before page end
    fn get_render_errors(&mut self) -> Vec<String> {
        vec![]
    }

    /// End the render and return the output
    fn end_render(&mut self, context: &mut Context) -> Output;
}
