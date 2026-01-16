use fs_err as fs;
use image::{RgbaImage, open};
use resvg::{tiny_skia, usvg};

fn load_image(path: &str) -> RgbaImage {
    let extension = path.rsplit('.').next().unwrap_or("").to_string();

    match extension.as_str() {
        "png" => open(path).unwrap().to_rgba8(),
        "svg" => svg_to_rgba(path.as_ref()),
        "html" => panic!("image from html not implemented or whatever"),
        _ => panic!("unknown extension/format type"),
    }
}

fn svg_to_rgba(path: &std::path::Path) -> RgbaImage {
    let svg_data = fs::read(path).unwrap();
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();

    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec()).unwrap()
}

pub fn check_files_equality(output: &str, expected_output: &str) {
    let result =
        image_compare::rgba_hybrid_compare(&load_image(output), &load_image(expected_output))
            .expect("Images had different dimensions");
    assert!(result.score > 0.95, "similarity too low: {}", result.score);
}
