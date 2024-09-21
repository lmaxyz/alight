use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl
};

use crate::MainWindow;
use slint::{Image as SlintImg, Weak};

pub struct Preview {
    title: String,
    index: usize,
    main_window: Weak<MainWindow>
}


impl GraphicsCaptureApiHandler for Preview {
    // To Get The Message From The Settings
    type Flags = (String, usize, Weak<MainWindow>);

    // To Redirect To CaptureControl Or Start Method
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function That Will Be Called To Create The Struct The Flags Can Be Passed
    // From `WindowsCaptureSettings`
    fn new(monitors_data: Self::Flags) -> Result<Self, Self::Error> {
        let title = monitors_data.0;
        let index = monitors_data.1;
        let main_window = monitors_data.2;

        Ok(Preview{
            title,
            index,
            main_window
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

        let title: slint::SharedString = self.title.as_str().into();
        let index = self.index as i32;
        let buff = slint::SharedPixelBuffer::clone_from_slice(&raw_buffer, width, height);

        let _ = self.main_window.upgrade_in_event_loop(move |w| {
            let slint_img = SlintImg::from_rgba8(buff);
            w.invoke_update_monitor_preview(index, title, slint_img);
        });

        #[cfg(not(debug_assertions))]
        std::thread::sleep(std::time::Duration::from_millis(25));

        #[cfg(debug_assertions)]
        std::thread::sleep(std::time::Duration::from_millis(50));

        Ok(())
    }

    // Called When The Capture Item Closes Usually When The Window Closes, Capture
    // Session Will End After This Function Ends
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Preview for {} monitor stopped", self.title);

        Ok(())
    }
}
