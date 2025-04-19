use log::debug;
use sdl2::EventPump;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::sys::{SDL_CreateTexture, SDL_PixelFormat, SDL_TextureAccess};
use sdl2::video::Window;

use sdl2::render::WindowCanvas;

use crate::{GAMEBOY_HEIGHT, GAMEBOY_WIDTH};

pub(crate) struct GameboyRenderer {
    pub canvas: WindowCanvas,
    current_scanline: u8,
    frame_elapsed_dots: u32,
    scanline_elapsed_dots: u32,
    current_pixel: u8,
    pub current_display: [u8; 23040],
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

pub struct SdlBackend {
    sdl_context: sdl2::Sdl,
    video_subsystem: sdl2::VideoSubsystem,
}
pub struct WindowDetails {
    title: String,
    width: u32,
    height: u32,
}

impl WindowDetails {
    pub fn new(title: String, width: u32, height: u32) -> Self {
        Self {
            title,
            width,
            height,
        }
    }
}
impl SdlBackend {
    pub fn new() -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        Ok(Self {
            sdl_context,
            video_subsystem,
        })
    }
    pub fn get_window(&mut self, windet: WindowDetails) -> Result<sdl2::video::Window, String> {
        self.video_subsystem
            .window(&windet.title, windet.width, windet.height)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())
    }
    pub fn get_event_pump(&mut self) -> Result<EventPump, String> {
        self.sdl_context.event_pump()
    }
}

impl GameboyRenderer {
    pub(crate) fn new(sdl_backend: &mut SdlBackend) -> Result<Self, String> {
        let window = sdl_backend.get_window(WindowDetails::new(
            "Gameboy".to_owned(),
            crate::GAMEBOY_WIDTH as u32,
            crate::GAMEBOY_HEIGHT as u32,
        ))?;
        let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        canvas.set_draw_color(Color::RGB(0x64, 0x95, 0xED));
        canvas.clear();
        canvas.present();
        Ok(Self {
            canvas,
            current_scanline: 0u8,
            frame_elapsed_dots: 0u32,
            scanline_elapsed_dots: 0u32,
            current_pixel: 0u8,
            current_display: [0u8; 160 * 144],
        })
    }
    pub fn current_display_to_texture(&mut self) {
        let tex_creator = self.canvas.texture_creator();
        let mut texture = tex_creator
            .create_texture_target(
                PixelFormatEnum::RGBA8888,
                GAMEBOY_WIDTH as u32,
                GAMEBOY_HEIGHT as u32,
            )
            .expect("unable to create texture");
        self.canvas
            .with_texture_canvas(&mut texture, |texture_canvas| {
                for i in 0..self.current_display.len() {
                    let x = (i % crate::GAMEBOY_WIDTH) as i32;
                    let y = (i / crate::GAMEBOY_WIDTH) as i32;
                    let i_val = self.current_display[i];
                    let draw_color = match i_val {
                        0 => Color::WHITE,
                        1 => Color::RED,
                        _ => Color::RED,
                    };
                    texture_canvas.set_draw_color(draw_color);
                    texture_canvas
                        .draw_point(Point::new(x, y))
                        .expect("Unable to draw point");
                }
            })
            .expect("Unable to draw to texture");
        self.canvas
            .copy(&texture, None, None)
            .expect("Unable to copy texture to canvas");
        self.canvas.present();
    }
    pub fn render_current_display(&mut self) {
        self.current_display_to_texture();
    }

    pub fn tick_dot(&mut self, lcdc_flags: RendererLcdcFlags, gb_memory: [u8; 0xffff + 1]) {
        self.current_display = [1u8; 23040];
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

    pub fn construct_pixel_array(
        &self,
        lcdc_flags: RendererLcdcFlags,
        gb_memory: [u8; 0xffff + 1],
    ) -> [u8; 256 * 256] {
        //TODO: Actually implement!
        [1u8; 256 * 256]
    }

    pub(crate) fn render_bg(&mut self, lcdc_flags: RendererLcdcFlags, gb_memory: [u8; 0xFFFF + 1]) {
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
