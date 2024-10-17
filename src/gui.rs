use fltk::{
    app::{self, App, Receiver, Scheme},
    button::Button,
    enums::{ColorDepth, FrameType, Shortcut},
    frame::Frame,
    image::RgbImage,
    prelude::*,
    window::Window,
};
use fltk_grid::Grid;
use image::{DynamicImage, GenericImage};
use std::{fs, path::Path};
use thiserror::Error;

const THUMB_SIZE: u32 = 384;
const FRAME_SIZE: i32 = (5 * THUMB_SIZE / 4) as i32;
const BUTTON_SIZE: i32 = 50;

/// Main GUI struct.
#[derive(Debug)]
pub struct GUI {
    app: App,
    receiver: Receiver<Message>,
    frame_l: Frame,
    frame_r: Frame,
    idx: usize,
    duplicates: Vec<(String, String)>,
}

/// GUI Events
#[derive(Clone, Copy, Debug)]
enum Message {
    LeftPressed,
    CenterPressed,
    RightPressed,
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

    /// Wrapper around [`std::io::Error`]
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

/// Simple result wrapper.
pub type Result<T> = std::result::Result<T, GUIError>;

/// Load an image from the filesystem and convert it to a thumbnail-sized FLTK
/// image.
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
        // square
        0 => embed.copy_from(&img, 0, 0)?,
        // landscape
        1.. => embed.copy_from(&img, 0, THUMB_SIZE / 2 - img.height() / 2)?,
    };

    Ok(RgbImage::new(
        embed.as_bytes(),
        THUMB_SIZE as i32,
        THUMB_SIZE as i32,
        ColorDepth::Rgb8,
    )?)
}

fn display_image<P: AsRef<Path>>(f: &mut Frame, file: P) -> Result<()> {
    f.set_image(Some(load_image(&file)?));
    f.set_label(file.as_ref().file_name().unwrap().to_str().unwrap());
    f.redraw();
    Ok(())
}

impl GUI {
    /// Create a new GUI.
    pub fn build(duplicates: Vec<(String, String)>) -> Result<Self> {
        let app = App::default().with_scheme(Scheme::Gtk);
        let mut win = Window::default()
            .with_size(FRAME_SIZE * 2, FRAME_SIZE + BUTTON_SIZE);
        win.size_range(
            THUMB_SIZE as i32 * 2,
            THUMB_SIZE as i32 + BUTTON_SIZE,
            0,
            0,
        );
        win.make_resizable(true);
        let mut grid = Grid::default_fill();
        let (s, receiver) = app::channel();

        // Define widgets
        let mut frame_l = Frame::default()
            .with_label("Left")
            .with_size(FRAME_SIZE, FRAME_SIZE);
        frame_l.set_frame(FrameType::ThinDownFrame);
        display_image(&mut frame_l, &duplicates[0].0)?;

        let mut frame_r = Frame::default()
            .with_label("Right")
            .with_size(FRAME_SIZE, FRAME_SIZE);
        frame_r.set_frame(FrameType::ThinDownFrame);
        display_image(&mut frame_r, &duplicates[0].1)?;

        let mut button_l = Button::default()
            .with_label("Keep left")
            .with_size(1, BUTTON_SIZE);
        button_l.set_shortcut(Shortcut::from_char('1'));
        button_l.emit(s, Message::LeftPressed);

        let mut button_c = Button::default()
            .with_label("Keep both")
            .with_size(1, BUTTON_SIZE);
        button_c.set_shortcut(Shortcut::from_char('2'));
        button_c.emit(s, Message::CenterPressed);

        let mut button_r = Button::default()
            .with_label("Keep right")
            .with_size(1, BUTTON_SIZE);
        button_r.set_shortcut(Shortcut::from_char('3'));
        button_r.emit(s, Message::RightPressed);

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
            receiver,
            frame_l,
            frame_r,
            idx: 0,
            duplicates,
        })
    }

    /// Run the GUI program. Consumes the program.
    pub fn run(mut self) -> Result<()> {
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                self.idx += 1;

                let (img_1, img_2) = match self.duplicates.get(self.idx) {
                    Some(dup) => (&dup.0, &dup.1),
                    None => break,
                };

                if !fs::exists(&img_1)? || !fs::exists(&img_2)? {
                    continue;
                }

                match msg {
                    Message::LeftPressed => {
                        display_image(&mut self.frame_l, &img_1)?;
                        display_image(&mut self.frame_r, &img_2)?;
                    }
                    Message::CenterPressed => {
                        display_image(&mut self.frame_l, &img_1)?;
                        display_image(&mut self.frame_r, &img_2)?;
                    }
                    Message::RightPressed => {
                        display_image(&mut self.frame_l, &img_1)?;
                        display_image(&mut self.frame_r, &img_2)?;
                    }
                }
            }
        }
        Ok(())
    }
}
