pub type Pixel = [u8; 4]; // RGBA format
pub type ScanLine = Vec<Pixel>;

pub fn create_empty_scanline(width: usize) -> ScanLine {
    vec![[255, 255, 255, 255]; width]
}

pub fn map_palette_color_to_rgba(color_index: u8) -> Pixel {
    match color_index {
        0 => [255, 255, 255, 255], // White
        1 => [170, 170, 170, 255], // Light Gray
        2 => [85, 85, 85, 255],    // Dark Gray
        3 => [0, 0, 0, 255],       // Black
        _ => [255, 0, 255, 255],   // Magenta (error)
    }
}
