use ring_channel::RingSender;
use tracing::debug;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl
};

use slint::{Rgba8Pixel, SharedPixelBuffer};

pub struct Preview {
    index: i32,
    preview_channel: RingSender<(i32, SharedPixelBuffer<Rgba8Pixel>)>
}


impl GraphicsCaptureApiHandler for Preview {
    // To Get The Message From The Settings
    type Flags = (usize, RingSender<(i32, SharedPixelBuffer<Rgba8Pixel>)>);

    // To Redirect To CaptureControl Or Start Method
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function That Will Be Called To Create The Struct The Flags Can Be Passed
    // From `WindowsCaptureSettings`
    fn new(monitors_data: Self::Flags) -> Result<Self, Self::Error> {
        let index = monitors_data.0 as i32;
        let preview_channel = monitors_data.1;

        Ok(Preview{
            index,
            preview_channel
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut frame_buffer = frame.buffer().unwrap();
        let width = frame_buffer.width();
        let height = frame_buffer.height();
        let raw_buffer = Vec::from(frame_buffer.as_raw_buffer());

        let buff = slint::SharedPixelBuffer::clone_from_slice(&raw_buffer, width, height);

        self.preview_channel.send((self.index, buff)).unwrap();

        #[cfg(not(debug_assertions))]
        std::thread::sleep(std::time::Duration::from_millis(5));

        #[cfg(debug_assertions)]
        std::thread::sleep(std::time::Duration::from_millis(25));

        Ok(())
    }

    // Called When The Capture Item Closes Usually When The Window Closes, Capture
    // Session Will End After This Function Ends
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        debug!("Preview #{} stopped", self.index);

        Ok(())
    }
}
