use crossterm::{
    cursor::{Hide, MoveTo, MoveToNextLine, Show},
    style::{Print, SetBackgroundColor},
    terminal::{Clear, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::Write;
use v4l::buffer::Type;
use v4l::format::fourcc::FourCC;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::traits::Capture;

fn main() {
    let _g = Guard;
    // Choose first caputre device
    let mut dev = Device::new(0).expect("Failed to open device");
    // Read device current format
    let mut fmt = dev.format().expect("Failed to read format");
    let width = fmt.width as usize;
    let height = fmt.height as usize;
    // Set device format to YUUV
    fmt.fourcc = FourCC::new(b"YUYV");
    dev.set_format(&fmt).expect("Failed to write format");

    // Create the capture stream
    let mut stream = MmapStream::with_buffers(&mut dev, Type::VideoCapture, 4)
        .expect("Failed to create buffer stream");

    crossterm::terminal::enable_raw_mode().unwrap();
    crossterm::execute!(std::io::stdout(), EnterAlternateScreen).unwrap();

    crossterm::execute!(std::io::stdout(), Hide).unwrap();
    let (w2, h2) = crossterm::terminal::size().unwrap();
    let w2 = w2 as usize;
    let h2 = h2 as usize;

    use std::sync::Arc;
    let exit_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ef = exit_flag.clone();

    std::thread::spawn(move || {
        if let Ok(_) = crossterm::event::read() {
            ef.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });

    while !exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
        crossterm::queue!(std::io::stdout(), MoveTo(0, 0)).unwrap();
        crossterm::queue!(
            std::io::stdout(),
            Clear(crossterm::terminal::ClearType::All)
        )
        .unwrap();

        // Read next video frame
        let (frame, _m) = stream.next().unwrap();
        // The video is encoded as Vec<u32> but we have a Vec<u8>
        // We read it 4 bytes at a time, in order to be processed correctly
        #[allow(non_snake_case)]
        let frame: Vec<u8> = frame.chunks(4).fold(vec![], |mut acc, v| {
            // convert form YUYV to RGB
            let [Y, U, _, V]: [u8; 4] = std::convert::TryFrom::try_from(v).unwrap();
            let Y = Y as f32;
            let U = U as f32;
            let V = V as f32;

            let b = 1.164 * (Y - 16.) + 2.018 * (U - 128.);

            let g = 1.164 * (Y - 16.) - 0.813 * (V - 128.) - 0.391 * (U - 128.);

            let r = 1.164 * (Y - 16.) + 1.596 * (V - 128.);
            let r = r as u8;
            let g = g as u8;
            let b = b as u8;
            acc.push(r);
            acc.push(g);
            acc.push(b);
            acc
        });

        use resize::Pixel::RGB24;
        use resize::Type::Lanczos3;

        let mut dst = vec![0; w2 * h2 * 3];

        let mut resizer = resize::new(width / 2, height, w2, h2, RGB24, Lanczos3);
        resizer.resize(&frame, &mut dst);

        dst.chunks(3).enumerate().for_each(|(idx, v)| {
            let r = v[0];
            let g = v[1];
            let b = v[2];

            if idx != 0 && idx % w2 == 0 {
                crossterm::queue!(std::io::stdout(), MoveToNextLine(1)).unwrap();
            }
            crossterm::queue!(
                std::io::stdout(),
                SetBackgroundColor(crossterm::style::Color::Rgb { r, g, b })
            )
            .unwrap();
            crossterm::queue!(std::io::stdout(), Print(" ")).unwrap();
        });
        std::io::stdout().flush().unwrap();
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        crossterm::terminal::disable_raw_mode().unwrap();
        crossterm::execute!(std::io::stdout(), LeaveAlternateScreen).unwrap();
        crossterm::execute!(std::io::stdout(), Show).unwrap();
    }
}
