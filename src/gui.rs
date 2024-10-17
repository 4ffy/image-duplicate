use fltk::{
    app::{App, Scheme},
    button::Button,
    enums::ColorDepth,
    frame::Frame,
    image::RgbImage,
    prelude::*,
    window::Window,
};
use fltk_grid::Grid;
use image::{DynamicImage, GenericImage};
use std::path::Path;
use thiserror::Error;

const THUMB_SIZE: u32 = 384;

/// eat
#[derive(Debug)]
pub struct GUI {
    app: App,
    win: Window,
    frame_l: Frame,
    frame_r: Frame,
    button_l: Button,
    button_c: Button,
    button_r: Button,
    idx: usize,
    duplicates: Vec<(String, String)>,
}

/// Errors that may occur when dealing with [`GUI`].
#[derive(Debug, Error)]
pub enum GUIError {
    /// Wrapper around [`fltk::prelude::FLTKError`]
    #[error("FLTK error: {0}")]
    FltkError(#[from] fltk::prelude::FltkError),

    /// Wrapper around [`image::ImageError`]
    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),
}

/// Simple result wrapper.
pub type Result<T> = std::result::Result<T, GUIError>;

fn load_image<P: AsRef<Path>>(file: P) -> Result<RgbImage> {
    assert!(file.as_ref().is_file());
    let img = image::open(file)?
        .thumbnail(THUMB_SIZE, THUMB_SIZE)
        .to_rgba8();
    let mut embed = DynamicImage::new_rgb8(THUMB_SIZE, THUMB_SIZE);

    // Embed differently based on with or height larger
    match img.width() as isize - img.height() as isize {
        // portrait
        ..=-1 => embed.copy_from(&img, THUMB_SIZE / 2 - img.width() / 2, 0)?,
        // landscape
        1.. => embed.copy_from(&img, 0, THUMB_SIZE / 2 - img.height() / 2)?,
        // square
        0 => embed.copy_from(&img, 0, 0)?,
    };

    Ok(RgbImage::new(
        embed.as_bytes(),
        THUMB_SIZE as i32,
        THUMB_SIZE as i32,
        ColorDepth::Rgb8,
    )?)
}

fn display_image<P: AsRef<Path>>(f: &mut Frame, file: P) {
    f.set_image(Some(load_image(&file).unwrap()));
    f.set_label(file.as_ref().file_name().unwrap().to_str().unwrap());
    f.redraw();
}

impl GUI {
    /// Create a new GUI.
    pub fn build(duplicates: Vec<(String, String)>) -> Result<Self> {
        let app = App::default().with_scheme(Scheme::Gtk);
        let mut win = Window::default()
            .with_size(THUMB_SIZE as i32 * 2, THUMB_SIZE as i32 + 100);
        let mut grid = Grid::default_fill();

        // Define widgets
        let mut frame_l = Frame::default()
            .with_label("Left")
            .with_size(THUMB_SIZE as i32, THUMB_SIZE as i32 + 50);
        let mut frame_r = Frame::default()
            .with_label("Right")
            .with_size(THUMB_SIZE as i32, THUMB_SIZE as i32 + 50);
        let mut button_l =
            Button::default().with_label("Keep left").with_size(1, 50);
        let mut button_c =
            Button::default().with_label("Keep both").with_size(1, 50);
        let mut button_r =
            Button::default().with_label("Keep right").with_size(1, 50);

        // Define grid
        grid.set_layout(2, 6);
        for col in 0..6 {
            grid.set_col_weight(col, 1);
        }
        grid.set_row_weight(0, 1);
        grid.set_row_weight(1, 0);

        // Grid widgets
        grid.set_widget(&mut frame_l, 0, 0..3)?;
        grid.set_widget(&mut frame_r, 0, 3..6)?;
        grid.set_widget(&mut button_l, 1, 0..2)?;
        grid.set_widget(&mut button_c, 1, 2..4)?;
        grid.set_widget(&mut button_r, 1, 4..6)?;

        // Finalize
        grid.end();
        win.end();
        win.show();

        Ok(Self {
            app,
            win,
            frame_l,
            frame_r,
            button_l,
            button_c,
            button_r,
            idx: 0,
            duplicates,
        })
    }

    /// Run the GUI program.
    pub fn run(&mut self) -> Result<()> {
        // Note: FLTK widgets are reference counted so cloning them is fine and
        // cheap.
        self.button_l.set_callback({
            let mut frame_l = self.frame_l.clone();
            move |_| {
                println!("Left button pushed!");
                display_image(&mut frame_l, "/home/cameron/Pictures/Memes2/4399ad7c10a53a7e2c027690c9eccc1a.jpg");
            }
        });

        self.button_r.set_callback({
            let mut frame_r = self.frame_r.clone();
            move |_| {
                println!("Right button pushed!");
                display_image(&mut frame_r, "/home/cameron/Pictures/Memes2/64ea577d9e55387200243e2873f05efd.jpg");
            }
        });

        self.button_c.set_callback({
            move |_| {
                println!("Center button pushed!");
            }
        });

        self.app.run()?;
        Ok(())
    }
}
