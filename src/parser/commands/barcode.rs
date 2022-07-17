extern crate barcoders;

use barcoders::sym::code128::Code128;
use std::str::from_utf8;

use crate::parser::*;

#[derive(Clone)]
enum BarcodeType {
  UpcA,
  UpcE,
  Ean13,
  Ean8,
  Code39,
  Itf,
  Nw7Codabar,
  Code93,
  Code128,
  Gs1128,
  Gs1DatabarOmni,
  Gs1DatabarTruncated,
  Gs1DatabarLimited,
  Gs1DatabarExpanded,
  Code128Auto,
  Unknown
}

#[derive(Clone)]
enum EncodingFunction {
  NulTerminated,
  ExplicitSize,
  Unknown
}

#[derive(Clone)]
struct BarcodeHandler{
  kind: BarcodeType,
  kind_id: u8,
  encoding: EncodingFunction,
  capacity: u8,
  has_capacity: bool,
  accept_data: bool
}

impl CommandHandler for BarcodeHandler {
  fn push(&mut self, data: &mut Vec<u8>, byte: u8) -> bool{ 
    let data_len = data.len();

    //Gather metadata
    if !self.accept_data {
      self.kind_id = byte;
      self.kind = match self.kind_id {
        0 => BarcodeType::UpcA,
        1 => BarcodeType::UpcE,
        2 => BarcodeType::Ean13,
        3 => BarcodeType::Ean8,
        4 => BarcodeType::Code39,
        5 => BarcodeType::Itf,
        6 => BarcodeType::Nw7Codabar,
        72 => BarcodeType::Code93,
        73 => BarcodeType::Code128,
        80 => BarcodeType::Gs1128,
        81 => BarcodeType::Gs1DatabarOmni,
        82 => BarcodeType::Gs1DatabarTruncated,
        83 => BarcodeType::Gs1DatabarLimited,
        84 => BarcodeType::Gs1DatabarExpanded,
        85 => BarcodeType::Code128Auto,
        _ => BarcodeType::Unknown
      };

      //I'm seeing some conflicting implementations for function definitions
      if byte <= 6 { self.encoding = EncodingFunction::NulTerminated; }
      else if byte >= 41 { self.encoding = EncodingFunction::ExplicitSize; } 
      else { self.encoding = EncodingFunction::Unknown }
      self.accept_data = true;
      
      return true;
    }

    match self.encoding {
      EncodingFunction::NulTerminated => {
        if *data.last().unwrap_or(&0x01u8) == 0x00u8 { 
          data.pop();
          return false 
        }
        data.push(byte);
        return true;
      },
      EncodingFunction::ExplicitSize => {
        if !self.has_capacity {
          self.capacity = byte;
          self.has_capacity = true;
          return true;
        } else if data_len < self.capacity as usize {
          data.push(byte);
          return true;
        }
        return false;
      },
      EncodingFunction::Unknown => return false,
    }
  }


  fn get_barcode(&self, command: &Command) -> Option<AbstractBarcode> {
    from_utf8(&command.data as &[u8]).unwrap_or("[No Data]");

    match self.kind {
        BarcodeType::Code128 => {
          //TODO ask the maintainers of the barcoders project if there is a better way to do this by passing in the native string with commands
          //It seems there is an existing control character for switching command pages which differs from what the maintainers implemented
          //but I actually have no idea what is right here
          let data = from_utf8(&command.data as &[u8]).unwrap_or("");
          if let Ok(barcode) = Code128::new(data.to_string()) {
            return Some(AbstractBarcode{
              lines: barcode.encode(),
              text: data.to_string()
            });
          } else {
            return None
          }
        }
        _ => return None
    }
  }

  fn debug(&self, command: &Command) -> String {
    let encoding_str = match self.encoding {
        EncodingFunction::NulTerminated => "Nul Terminated",
        EncodingFunction::ExplicitSize => "Explicit Size",
        EncodingFunction::Unknown => "Unknown",
    };

    let type_str = match self.kind {
        BarcodeType::UpcA => "UPC A",
        BarcodeType::UpcE => "UPC E",
        BarcodeType::Ean13 => "EAN 13",
        BarcodeType::Ean8 => "EAN 8",
        BarcodeType::Code39 => "CODE 39",
        BarcodeType::Itf => "ITF",
        BarcodeType::Nw7Codabar => "Nw7Codabar",
        BarcodeType::Code93 => "Code93",
        BarcodeType::Code128 => "Code128",
        BarcodeType::Gs1128 => "GS1 128",
        BarcodeType::Gs1DatabarOmni => "GS1 Omni",
        BarcodeType::Gs1DatabarTruncated => "GS1 Truncated",
        BarcodeType::Gs1DatabarLimited => "GS1 Kimited",
        BarcodeType::Gs1DatabarExpanded => "GS1 Expanded",
        BarcodeType::Code128Auto => "Code 128 Auto",
        BarcodeType::Unknown => "Unknown",
    };

    if matches!(self.kind, BarcodeType::Unknown) { 
      return format!("Unknown Barcode Format with {} encoding and a type id of {} and data {:02X?}", encoding_str, self.kind_id, command.data)
    } 
    format!("{} Barcode with data: {}", type_str, from_utf8(&command.data as &[u8]).unwrap_or("[No Data]") )
  }
}

pub fn new() -> Command {
  Command::new(
    "Barcode",
    vec![GS, 'k' as u8], 
    CommandType::Graphics,
    DataType::Custom,
    Box::new(BarcodeHandler{
      kind: BarcodeType::Unknown,
      kind_id: 0,
      encoding: EncodingFunction::Unknown,
      capacity: 0,
      has_capacity: false,
      accept_data: false
    })
  )
}