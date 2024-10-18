use fltk::{
    app::{self, App, Receiver, Scheme},
    button::Button,
    enums::{ColorDepth, FrameType, Shortcut},
    frame::Frame,
    group::Flex,
    image::RgbImage,
    prelude::*,
    window::Window,
};
use image::{DynamicImage, GenericImage};
use std::{fs, path::Path};
use thiserror::Error;
use trash;

const THUMB_SIZE: u32 = 384;
const FRAME_SIZE: i32 = (5 * THUMB_SIZE / 4) as i32;
const BUTTON_SIZE: i32 = 50;

/// Main GUI struct.
#[derive(Debug)]
pub struct GUI {
    app: App,
    win: Window,
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

    /// Wrapper around [`trash::Error`]
    #[error("Trash error: {0}")]
    TrashError(#[from] trash::Error),
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
    let size = image::image_dimensions(&file)?;
    let label = format!(
        "{} {size:?}",
        file.as_ref().file_name().unwrap().to_str().unwrap()
    );
    f.set_label(&label);
    f.set_image(Some(load_image(&file)?));
    Ok(())
}

impl GUI {
    /// Create a new GUI.
    pub fn build(duplicates: Vec<(String, String)>) -> Result<Self> {
        let (s, receiver) = app::channel();
        let app = App::default().with_scheme(Scheme::Gtk);

        let mut win = Window::default()
            .with_size(FRAME_SIZE * 2, FRAME_SIZE + BUTTON_SIZE);
        win.size_range(
            THUMB_SIZE as i32 * 2,
            THUMB_SIZE as i32 + BUTTON_SIZE + BUTTON_SIZE / 2,
            0,
            0,
        );
        win.make_resizable(true);

        let mut main = Flex::default().column().size_of_parent();

        let row1 = Flex::default().row();
        let mut frame_l = Frame::default().with_label("Left");
        let mut frame_r = Frame::default().with_label("Right");
        frame_l.set_frame(FrameType::ThinDownFrame);
        frame_r.set_frame(FrameType::ThinDownFrame);
        row1.end();

        let row2 = Flex::default().row();
        let mut button_l = Button::default().with_label("1: Keep left");
        let mut button_c = Button::default().with_label("2: Keep both");
        let mut button_r = Button::default().with_label("3: Keep right");
        button_l.emit(s, Message::LeftPressed);
        button_c.emit(s, Message::CenterPressed);
        button_r.emit(s, Message::RightPressed);
        button_l.set_shortcut(Shortcut::from_char('1'));
        button_c.set_shortcut(Shortcut::from_char('2'));
        button_r.set_shortcut(Shortcut::from_char('3'));
        row2.end();

        main.fixed(&row2, BUTTON_SIZE);

        main.end();

        win.end();

        Ok(Self {
            app,
            win,
            receiver,
            frame_l,
            frame_r,
            idx: 0,
            duplicates,
        })
    }

    /// Run the GUI program. Consumes the program.
    pub fn run(mut self) -> Result<()> {
        self.win.show();

        let (mut img_1, mut img_2) = match self.duplicates.get(self.idx) {
            Some(dup) => (&dup.0, &dup.1),
            None => return Ok(()),
        };
        display_image(&mut self.frame_l, &img_1)?;
        display_image(&mut self.frame_r, &img_2)?;

        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                match msg {
                    Message::LeftPressed => {
                        eprintln!("Trashing \"{img_2}\"");
                        trash::delete(&img_2)?;
                    }
                    Message::CenterPressed => {
                        eprintln!("Keeping both images");
                    }
                    Message::RightPressed => {
                        eprintln!("Trashing \"{img_1}\"");
                        trash::delete(&img_1)?;
                    }
                }

                self.idx += 1;
                (img_1, img_2) = match self.duplicates.get(self.idx) {
                    Some(dup) => (&dup.0, &dup.1),
                    None => return Ok(()),
                };

                while !fs::exists(&img_1)? || !fs::exists(&img_2)? {
                    self.idx += 1;
                    (img_1, img_2) = match self.duplicates.get(self.idx) {
                        Some(dup) => (&dup.0, &dup.1),
                        None => return Ok(()),
                    };
                }

                display_image(&mut self.frame_l, &img_1)?;
                display_image(&mut self.frame_r, &img_2)?;
                self.win.redraw();
            }
        }
        Ok(())
    }
}
