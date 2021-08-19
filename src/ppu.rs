use log::debug;
pub struct Ppu {
    vram: [u8; 0x2000],
    oam: [u8; 0xa0],
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    dma: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    frame: [u8; 160 * 144],
    counter: u16,
    irq_lcdc: bool,
    irq_vblank: bool,
}

enum MapArea {
    Base1800,
    Base1C00,
}

enum TileArea {
    Base1000,
    Base0000,
}

enum ObjSize {
    Square,
    Rectangle,
}

enum Mode {
    HBlank,       // Mode0
    VBlank,       // Mode1
    SearchingOAM, // Mode2
    Drawing,      // Mode3
}

impl Ppu {
    pub(crate) fn new() -> Self {
        Ppu {
            vram: [0; 0x2000],
            oam: [0; 0xa0],
            lcdc: 0x80,
            stat: 0x02,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            frame: [0; 160 * 144],
            counter: 0,
            irq_lcdc: false,
            irq_vblank: false,
        }
    }
    pub fn get_frame(&self) -> &[u8] {
        &self.frame
    }

    pub fn is_irq_vblank(&self) -> bool {
        self.irq_vblank
    }

    pub fn is_irq_lcdc(&self) -> bool {
        self.irq_lcdc
    }

    pub fn set_irq_vblank(&mut self, flag: bool) {
        self.irq_vblank = flag;
    }

    pub fn set_irq_lcdc(&mut self, flag: bool) {
        self.irq_lcdc = flag;
    }

    fn is_lcd_and_ppu_enable(&self) -> bool {
        ((self.lcdc >> 7) & 1) == 1
    }

    fn window_map_area(&self) -> MapArea {
        match ((self.lcdc >> 6) & 1) == 1 {
            false => MapArea::Base1800,
            true => MapArea::Base1C00,
        }
    }

    fn is_window_enable(&self) -> bool {
        ((self.lcdc >> 5) & 1) == 1
    }

    fn get_tile_area(&self) -> TileArea {
        match ((self.lcdc >> 4) & 1) == 1 {
            false => TileArea::Base1000,
            true => TileArea::Base0000,
        }
    }

    fn bg_map_area(&self) -> MapArea {
        match ((self.lcdc >> 3) & 1) == 1 {
            false => MapArea::Base1800,
            true => MapArea::Base1C00,
        }
    }

    fn get_obj_size(&self) -> ObjSize {
        match ((self.lcdc >> 2) & 1) == 1 {
            false => ObjSize::Square,
            true => ObjSize::Rectangle,
        }
    }

    fn is_obj_enable(&self) -> bool {
        ((self.lcdc >> 1) & 1) == 1
    }

    fn is_bg_window_enable(&self) -> bool {
        (self.lcdc & 1) == 1
    }

    fn is_lyc_eq_ly_stat_interrupt(&self) -> bool {
        ((self.stat >> 6) & 1) == 1
    }

    fn is_mode2_oam_stat_interrupt(&self) -> bool {
        ((self.stat >> 5) & 1) == 1
    }

    fn is_mode1_vblank_stat_interrupt(&self) -> bool {
        ((self.stat >> 4) & 1) == 1
    }

    fn is_mode0_hblank_stat_interrupt(&self) -> bool {
        ((self.stat >> 3) & 1) == 1
    }

    fn is_lcy_eq_ly_flag(&self) -> bool {
        ((self.stat >> 2) & 1) == 1
    }

    fn get_mode_flag(&self) -> Mode {
        match self.stat & 0x03 {
            0 => Mode::HBlank,
            1 => Mode::VBlank,
            2 => Mode::SearchingOAM,
            3 => Mode::Drawing,
            _ => panic!("Invalid mode: {}", self.stat & 0x03),
        }
    }

    fn set_mode_flag(&mut self, mode: Mode) {
        match mode {
            Mode::HBlank => self.stat = self.stat & 0xf8,
            Mode::VBlank => self.stat = (self.stat & 0xf8) | 1,
            Mode::SearchingOAM => self.stat = (self.stat & 0xf8) | 2,
            Mode::Drawing => self.stat = (self.stat & 0xf8) | 3,
        }
    }

    fn is_sprite_visible(&self, sprite_x: u8, sprite_y: u8, height: u8) -> bool {
        (0 < sprite_x) && (sprite_x <= 160 + 7) &&                       // x condition
        (sprite_y <= self.ly + 16) && (self.ly + 16 < sprite_y + height) // y condition
    }

    fn get_bg_window_tile_row(
        &self,
        tile_x: u8,
        tile_y: u8,
        offset_y: u8,
        window_flag: bool,
    ) -> (u8, u8) {
        let tile_map_index = (tile_x as u16) + (tile_y as u16) * 32;
        let tile_map_addr = if window_flag {
            match self.window_map_area() {
                MapArea::Base1800 => 0x1800 + tile_map_index,
                MapArea::Base1C00 => 0x1C00 + tile_map_index,
            }
        } else {
            match self.bg_map_area() {
                MapArea::Base1800 => 0x1800 + tile_map_index,
                MapArea::Base1C00 => 0x1C00 + tile_map_index,
            }
        };

        let tile_no = self.vram[tile_map_addr as usize];
        let mut tile_addr = match self.get_tile_area() {
            TileArea::Base0000 => (tile_no as u16) * 16,
            TileArea::Base1000 => (0x1000 as i16).wrapping_add((tile_no as i8 as i16) * 16) as u16,
        };
        tile_addr = tile_addr.wrapping_add((offset_y as u16) * 2);

        let tile_row_low = self.vram[tile_addr as usize];
        let tile_row_high = self.vram[(tile_addr + 1) as usize];
        debug!(
            "tile_map_addr: {}, tile_low: {}, tile_high: {}",
            tile_addr, tile_row_low, tile_row_high
        );

        (tile_row_low, tile_row_high)
    }

    fn get_sprite_tile_row(&mut self, tile_no: u8, offset_y: u8) -> (u8, u8) {
        let tile_addr = tile_no * 16 + offset_y * 2;
        let tile_row_low = self.vram[tile_addr as usize];
        let tile_row_high = self.vram[(tile_addr + 1) as usize];

        (tile_row_low, tile_row_high)
    }

    fn get_pixel_color(&self, tile_row_low: u8, tile_row_high: u8, offset_x: u8) -> u8 {
        let tile_color = self.get_tile_color(tile_row_low, tile_row_high, offset_x);

        match (self.bgp >> (tile_color << 1)) & 0x3 {
            0 => 0xff,
            1 => 0xaa,
            2 => 0x55,
            3 => 0x00,
            _ => panic!("Invalid tile_color: {}", tile_color),
        }
    }

    fn get_sprite_color(&mut self, tile_color: u8, sprite_flag: u8) -> u8 {
        let palette = if sprite_flag & 0x10 > 0 {
            self.obp1
        } else {
            self.obp0
        };

        match (palette >> (tile_color << 1)) & 0x3 {
            0 => 0xff,
            1 => 0xaa,
            2 => 0x55,
            3 | _ => 0x00,
        }
    }

    fn get_tile_color(&self, tile_row_low: u8, tile_row_high: u8, offset_x: u8) -> u8 {
        let shift_num = 7 - offset_x;
        let bit_low = (tile_row_low >> shift_num) & 1;
        let bit_high = (tile_row_high >> shift_num) & 1;

        bit_high << 1 | bit_low
    }

    fn render_bg(&mut self) {
        let wx = self.wx.wrapping_sub(7);
        let wy = self.wy;

        for x in 0..160 {
            let window_flag = (wy <= self.ly) && (wx <= self.scx + x) && (self.is_window_enable());

            let mut pixel_x = 0;
            let mut pixel_y = 0;
            if window_flag {
                pixel_x = self.scx + x - wx;
                pixel_y = self.ly - wy;
            } else {
                pixel_x = self.scx.wrapping_add(x);
                pixel_y = self.scy.wrapping_add(self.ly);
            }

            let tile_x = pixel_x >> 3;
            let tile_y = pixel_y >> 3;
            let offset_x = pixel_x & 0x07;
            let offset_y = pixel_y & 0x07;

            let (tile_row_low, tile_row_high) =
                self.get_bg_window_tile_row(tile_x, tile_y, offset_y, window_flag);

            let color = self.get_pixel_color(tile_row_low, tile_row_high, offset_x);
            let index = (x as usize) + (self.ly as usize) * 160;
            debug!(
                "render scan tile_x: {}, tile_y: {}, offset_x: {}, offset_y: {}, x: {}, color: {}",
                tile_x, tile_y, offset_x, offset_y, x, color
            );
            debug!(
                "tile_low, tile_high: {}, {}, window_flag: {}",
                tile_row_low, tile_row_high, window_flag
            );
            self.frame[index] = color;
        }
    }

    fn render_sprites(&mut self) {
        let mut sprites_num = 0;
        let height = if self.lcdc & 0x4 > 0 { 16 } else { 8 };

        for i in 0..40 {
            let sprite_addr = i * 4;

            let sprite_y = self.oam[sprite_addr];
            let sprite_x = self.oam[sprite_addr + 1];
            let tile_index = self.oam[sprite_addr + 2];
            let sprite_flag = self.oam[sprite_addr + 3];

            let bg_window_priority_flag = sprite_flag & 0x80 > 0;
            let flip_y_flag = sprite_flag & 0x40 > 0;
            let flip_x_flag = sprite_flag & 0x20 > 0;

            if !self.is_sprite_visible(sprite_x, sprite_y, height) {
                continue;
            }

            sprites_num += 1;
            if sprites_num > 10 {
                break;
            }

            let tile_no = if self.lcdc & 0x4 > 0 {
                if (self.ly + 8 < sprite_y) ^ flip_y_flag {
                    self.oam[sprite_addr + 2] & 0xfe
                } else {
                    self.oam[sprite_addr + 2] | 0x01
                }
            } else {
                self.oam[sprite_addr + 2]
            };

            let offset_y = if flip_y_flag {
                7 - ((self.ly + 16 - sprite_y) & 0x7)
            } else {
                (self.ly + 16 - sprite_y) & 0x7
            };

            let (tile_row_low, tile_row_high) = self.get_sprite_tile_row(tile_no, offset_y);

            for offset_x in 0..8 {
                if sprite_x + offset_x < 8 {
                    continue;
                }
                let pixel_x = sprite_x + offset_x - 8;
                if pixel_x >= 160 {
                    break;
                }
                let index_x = if flip_x_flag { 7 - offset_x } else { offset_x };
                let tile_color = self.get_tile_color(tile_row_low, tile_row_high, index_x);

                if tile_color == 0 {
                    continue;
                }
                if self.frame[pixel_x as usize] != 0 && bg_window_priority_flag {
                    continue;
                }
                let color = self.get_sprite_color(tile_color, sprite_flag);
                debug!("Sprite color: {}, x: {}", color, pixel_x);
                let index = (pixel_x as usize) + (self.ly as usize) * 160;
                self.frame[index] = color;
            }
        }
    }

    fn render_scan(&mut self) {
        if self.lcdc & 0x1 > 0 {
            self.render_bg();
        }
        if self.lcdc & 0x2 > 0 {
            self.render_sprites();
        }
    }

    pub(crate) fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9fff => {
                if self.stat & 0x3 != 3 {
                    self.vram[(addr & 0x1fff) as usize]
                } else {
                    0xff
                }
            }

            0xfe00..=0xfe9f => {
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 {
                    self.oam[(addr & 0x00ff) as usize]
                } else {
                    0xff
                }
            }

            // IO registers
            0xff40 => self.lcdc,
            0xff41 => self.stat,
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff46 => self.dma,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    pub(crate) fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9fff => {
                if self.stat & 0x3 != 3 {
                    debug!(
                        "VRAM write addr: 0x{:04x}, value: 0x{:02x}",
                        addr & 0x1fff,
                        value
                    );
                    self.vram[(addr & 0x1fff) as usize] = value
                }
            }

            0xfe00..=0xfe9f => {
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 {
                    self.oam[(addr & 0x00ff) as usize] = value;
                }
            }

            0xff40 => {
                if self.lcdc & 0x80 != value & 0x80 {
                    self.ly = 0;
                    self.counter = 0;

                    let mode = if value & 0x80 > 0 { 2 } else { 0 };
                    self.stat = (self.stat & 0xf8) | mode;
                    self.update_mode_interrupt();
                }

                self.lcdc = value;
            }
            0xff41 => self.stat = (value & 0xf8) | (self.stat & 0x3),
            0xff42 => self.scy = value,
            0xff43 => self.scx = value,
            0xff44 => (),
            0xff45 => {
                if self.lyc != value {
                    self.lyc = value;
                    self.update_lyc_interrupt();
                }
            }
            0xff47 => self.bgp = value,
            0xff48 => self.obp0 = value,
            0xff49 => self.obp1 = value,
            0xff4a => self.wy = value,
            0xff4b => self.wx = value,

            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn update_lyc_interrupt(&mut self) {
        // LYC=LY coincidence interrupt
        if self.ly == self.lyc {
            self.stat |= 0x4;
            self.irq_lcdc = true;
        } else {
            self.stat &= !0x4;
        }
    }

    /// Checks LCD mode interrupt.
    fn update_mode_interrupt(&mut self) {
        // Mode interrupts
        match self.stat & 0x3 {
            // H-Blank interrupt
            0 if self.stat & 0x8 > 0 => self.irq_lcdc = true,
            // V-Blank interrupt
            1 if self.stat & 0x10 > 0 => self.irq_lcdc = true,
            // OAM Search interrupt
            2 if self.stat & 0x20 > 0 => self.irq_lcdc = true,
            _ => (),
        }
    }

    pub(crate) fn update(&mut self, clock: u8) {
        debug!(
            "PPU update ly: {}, scx: {}, scy: {}",
            self.ly, self.scx, self.scy
        );
        debug!("lcdc: 0x{:02x}, stat: 0x{:02x}", self.lcdc, self.stat);
        debug!(
            "bgp: {}, obp0: {}, obp1: {}, wy: {}, wx: {}",
            self.bgp, self.obp0, self.obp1, self.wy, self.wx
        );
        debug!("mmu_clock: {}, update_clock: {}", self.counter, clock);

        if !self.is_lcd_and_ppu_enable() {
            debug!("LCD and PPU is not enable");
            return;
        }

        self.counter += clock as u16;

        match self.get_mode_flag() {
            Mode::SearchingOAM => {
                if self.counter >= 80 {
                    self.counter -= 80;
                    self.set_mode_flag(Mode::Drawing);
                    self.render_scan();
                    debug!("Render mode: searching oam");
                }
            }
            Mode::Drawing => {
                if self.counter >= 172 {
                    self.counter -= 172;
                    self.set_mode_flag(Mode::HBlank);
                    self.update_mode_interrupt();
                    debug!("Render mode: drawing");
                }
            }
            Mode::HBlank => {
                if self.counter >= 204 {
                    self.counter -= 204;
                    self.ly += 1;
                    if self.ly >= 144 {
                        self.set_mode_flag(Mode::VBlank);
                        self.irq_vblank = true;
                    } else {
                        self.set_mode_flag(Mode::SearchingOAM);
                    }
                    debug!("Render mode HBlank");

                    self.update_lyc_interrupt();
                    self.update_mode_interrupt();
                }
            }
            Mode::VBlank => {
                if self.counter >= 456 {
                    self.counter -= 456;
                    self.ly += 1;

                    if self.ly >= 154 {
                        self.set_mode_flag(Mode::SearchingOAM);
                        self.ly = 0;

                        self.update_mode_interrupt();
                    }

                    self.update_lyc_interrupt();
                    debug!("Render mode VBlank");
                }
            }
        }
    }
}
