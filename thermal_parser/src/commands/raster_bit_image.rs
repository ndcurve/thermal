use crate::{command::*, constants::*, context::*, graphics::*};

#[derive(Clone)]
struct Handler {
    width: u32,
    height: u32,
    capacity: u32,
    scaling: u8,
    accept_data: bool,
    params: Vec<u8>,
}

impl CommandHandler for Handler {
    fn get_graphics(&self, command: &Command, context: &Context) -> Option<GraphicsCommand> {
        let stretch = match self.scaling {
            0 | 48 => (1, 1),
            1 | 49 => (2, 1),
            2 | 50 => (1, 2),
            3 | 52 => (2, 2),
            _ => (1, 1),
        };

        Some(GraphicsCommand::image_from_raster_bytes_single_color(
            self.width,
            self.height,
            stretch,
            context.graphics.render_colors.color_for_number(1),
            ImageFlow::Block,
            &command.data,
            true,
        ))
    }
    fn push(&mut self, data: &mut Vec<u8>, byte: u8) -> bool {
        let data_len = data.len();

        if !self.accept_data {
            if data_len < 4 {
                data.push(byte);
                return true;
            }
            self.scaling = *data.get(0).unwrap();
            let xl = *data.get(1).unwrap() as u32;
            let xh = *data.get(2).unwrap() as u32;
            let yl = *data.get(3).unwrap() as u32;
            let yh = byte as u32;

            self.width = xl + xh * 256;
            self.height = yl + yh * 256;
            self.capacity = self.width * self.height;
            self.width = self.width * 8;

            self.params = vec![self.scaling, xl as u8, xh as u8, yl as u8, yh as u8];

            data.clear();

            self.accept_data = true;
            return true;
        }

        if data_len >= self.capacity as usize {
            return false;
        }
        data.push(byte);
        true
    }

    //Used when converting commands back into other formats i.e. Thermal format
    fn get_command_bytes(&self, command: &Command) -> (Vec<u8>, Vec<u8>) {
        let mut data = self.params.clone();
        let commands = command.commands.to_vec();
        data.extend(command.data.clone());
        (commands, data)
    }
}

pub fn new() -> Command {
    Command::new(
        "Raster Bit Image",
        vec![GS, 'v' as u8, '0' as u8],
        CommandType::Graphics,
        DataType::Custom,
        Box::new(Handler {
            width: 0,
            height: 0,
            capacity: 0,
            scaling: 0,
            accept_data: false,
            params: vec![],
        }),
    )
}
