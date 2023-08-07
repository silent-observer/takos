use crate::display::{ColorRGB, FrameBuffer};

const FONT_ARRAY: &[u8; 4266] = include_bytes!("../resources/iso-8x16.font");
const GLYPH_WIDTH: usize = 8;
const GLYPH_HEIGHT: usize = 16;

impl FrameBuffer {
    pub fn put_symbol(
        &self,
        x: usize,
        y: usize,
        fg_color: ColorRGB,
        bg_color: ColorRGB,
        symbol: u8,
    ) {
        assert!(x * GLYPH_WIDTH < self.width());
        assert!(y * GLYPH_HEIGHT < self.height());
        for row in 0..GLYPH_HEIGHT {
            let data = FONT_ARRAY[symbol as usize * GLYPH_HEIGHT + row];
            for col in 0..GLYPH_WIDTH {
                let color = if (data >> (7 - col)) & 1 != 0 {
                    fg_color
                } else {
                    bg_color
                };
                self.put_pixel(x * GLYPH_WIDTH + col, y * GLYPH_HEIGHT + row, color);
            }
        }
    }

    #[inline]
    pub fn text_width(&self) -> usize {
        self.width() / GLYPH_WIDTH
    }

    #[inline]
    pub fn text_height(&self) -> usize {
        self.height() / GLYPH_HEIGHT
    }

    #[inline]
    pub fn text_move_up(&self, offset: usize, fill_color: ColorRGB) {
        self.move_up(offset * GLYPH_HEIGHT, fill_color);
    }

    #[inline]
    pub fn text_move_down(&self, offset: usize, fill_color: ColorRGB) {
        self.move_down(offset * GLYPH_HEIGHT, fill_color);
    }
}
