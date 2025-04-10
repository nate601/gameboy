use sdl2::EventPump;
use sdl2::pixels::Color;

use sdl2::render::WindowCanvas;

pub(crate) struct Renderer {
    pub(crate) canvas: WindowCanvas,
    pub(crate) event_pump: EventPump,
    pub(crate) current_scanline: u8,
}

pub(crate) struct RendererLcdcFlags {
    pub(crate) lcd_enable: bool,
    pub(crate) window_tile_map: bool,
    pub(crate) window_enable: bool,
    pub(crate) bg_and_window_tiles: bool,
    pub(crate) bg_tile_map: bool,
    pub(crate) obj_size: bool,
    pub(crate) obj_enable: bool,
    pub(crate) bg_and_window_enable_priority: bool,
}

impl RendererLcdcFlags {
    pub(crate) fn new(byte: u8) -> Self {
        Self {
            lcd_enable: (byte & 0b10000000) > 0,
            window_tile_map: (byte & 0b0100000) > 0,
            window_enable: (byte & 0b0010000) > 0,
            bg_and_window_tiles: (byte & 0b00010000) > 0,
            bg_tile_map: (byte & 0b00001000) > 0,
            obj_size: (byte & 0b0000_0100) > 0,
            obj_enable: (byte & 0b0000_0010) > 0,
            bg_and_window_enable_priority: (byte & 0b0000_0001) > 0,
        }
    }
}

impl Renderer {
    pub(crate) fn renderer_init() -> Result<Renderer, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window("gameboy", 256, 256)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;
        let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let mut event_pump = sdl_context.event_pump()?;
        canvas.set_draw_color(Color::RGB(0x64, 0x95, 0xED));
        canvas.clear();
        canvas.present();
        Ok(Renderer {
            canvas,
            event_pump,
            current_scanline: 0u8,
        })
    }
    pub fn advance_scanline(&mut self) {
        let current_scanline = self.current_scanline;
        let mut new_scanline = current_scanline.saturating_add(1);
        if new_scanline >= 153 {
            new_scanline = 0;
        }
        self.current_scanline = new_scanline
    }
    pub fn render_next_scanline(
        &mut self,
        lcdc_flags: RendererLcdcFlags,
        gb_memory: [u8; 0x0FFFF + 1],
    ) {
        let current_scanline = self.current_scanline;
        match current_scanline {
            0..143 => {}
            143..153 => {}
            _ => panic!("Unexpected scanline number {current_scanline}"),
        }
        self.advance_scanline();
    }

    pub(crate) fn render_bg(
        &mut self,
        lcdc_flags: RendererLcdcFlags,
        gb_memory: [u8; 0x0FFFF + 1],
    ) {
        if !lcdc_flags.lcd_enable {
            //Return early, screen is disabled
            return ();
        }
        // go thru tilemap
        // then go thru tiles
        // render that shit
        let tilemap_base_location = if !lcdc_flags.bg_tile_map {
            0x9800
        } else {
            0x9c00
        };
        let tile_data = if !lcdc_flags.bg_and_window_tiles {
            &gb_memory[0x8000..0x8FFF]
        } else {
            &gb_memory[0x8800..0x97FF]
        };
        // println!("{:?}", tile_data);
        // for i in (0..tile_data.len()).step_by(16) {
        //
        // }

        //  start location -> 1023
        let tilemap_data = &gb_memory[tilemap_base_location..tilemap_base_location + 1023];
        // println!("{:?}", tilemap_data);
    }
}
