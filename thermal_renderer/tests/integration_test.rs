use std::path::PathBuf;
use thermal_parser::command::Command;
use thermal_parser::context::Context;
use thermal_parser::thermal_file::parse_str;
use thermal_renderer::html_renderer::HtmlRenderer;
use thermal_renderer::image_renderer::ImageRenderer;
use thermal_renderer::renderer::CommandRenderer;

#[test]
fn bad_image() {
    test_sample("bad_image", "thermal")
}

#[test]
fn code_pages() {
    test_sample("code_pages", "thermal")
}

#[test]
fn corrupt_start_of_binary() {
    test_sample("corrupt_start_of_binary", "bin")
}

#[test]
fn issuing_receipts() {
    test_sample("issuing_receipts", "thermal")
}

#[test]
fn page_mode() {
    test_sample("page_mode", "thermal")
}

#[test]
fn print_graphics() {
    test_sample("print_graphics", "thermal")
}

#[test]
fn receipt_with_barcode() {
    test_sample("receipt_with_barcode", "thermal")
}

#[test]
fn scale_test() {
    test_sample("scale_test", "bin")
}

#[test]
fn gs_images_column() {
    test_sample("gs_images_column", "bin")
}

#[test]
fn gs_images_raster() {
    test_sample("gs_images_raster", "bin")
}

#[test]
fn test_receipt_1() {
    test_sample("test_receipt_1", "bin")
}

#[test]
fn test_receipt_2() {
    test_sample("test_receipt_2", "bin")
}

#[test]
fn test_receipt_3() {
    test_sample("test_receipt_3", "bin")
}

#[test]
fn test_receipt_4() {
    test_sample("test_receipt_4", "bin")
}

#[test]
fn thick_barcode() {
    test_sample("thick_barcode", "bin")
}

fn test_sample(name: &str, ext: &str) {
    let sample_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("sample_files")
        .join("in")
        .join(format!("{}.{}", name, ext));

    let rendered_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("sample_files")
        .join("out")
        .join(format!("{}.{}", name, ext));

    let bytes = if ext == "thermal" {
        let text = std::fs::read_to_string(sample_file.to_str().unwrap()).unwrap();
        parse_str(&text)
    } else {
        std::fs::read(sample_file.to_str().unwrap()).unwrap()
    };

    println!("Render Test [{}]", name);
    render_image(&bytes, rendered_file.to_str().unwrap().to_string());
    render_html(&bytes, rendered_file.to_str().unwrap().to_string());
}

fn render_html(bytes: &Vec<u8>, out_path: String) {
    let mut html_renderer = HtmlRenderer::new(out_path);
    let mut context = Context::new();

    let on_new_command = move |mut cmd: Command| {
        html_renderer.process_command(&mut context, &mut cmd);
    };

    let mut command_parser = thermal_parser::new_esc_pos_parser(Box::from(on_new_command));
    command_parser.parse_bytes(bytes);
}

fn render_image(bytes: &Vec<u8>, out_path: String) {
    let mut image_renderer = ImageRenderer::new(out_path);

    let mut context = Context::new();

    let on_new_command = move |mut cmd: Command| {
        image_renderer.process_command(&mut context, &mut cmd);
    };

    let mut command_parser = thermal_parser::new_esc_pos_parser(Box::from(on_new_command));
    command_parser.parse_bytes(bytes);
}
