use std::{cell::RefCell, io::Cursor};
use std::error::Error;
use std::time::Duration;
use std::rc::Rc;

use serialport::{available_ports, new, SerialPortType};
use slint::{Model, SharedPixelBuffer, VecModel, Weak};
use windows_capture::{
    capture::{GraphicsCaptureApiHandler, CaptureControl},
    monitor::Monitor, settings::{ColorFormat, Settings},
    settings::{DrawBorderSettings, CursorCaptureSettings}
};
use image::{DynamicImage, ImageReader};

mod screen_capture;
mod preview;

use preview::Preview;
use screen_capture::Capture;

slint::include_modules!();

const MAX_TITLE_LEN: usize = 32;

type CaptureControlRef = Rc<RefCell<Option<CaptureControl<Capture, Box<dyn Error + Send + Sync>>>>>;


fn get_monitor_title(monitor: &Monitor) -> String {
    let name = monitor.name().expect("Can't get monitor title");
    let device_str = monitor.device_string().expect("Can't get device string from monitor");

    let mut title = format!("{} ({})", name, device_str);
    if title.chars().count() > MAX_TITLE_LEN {
        title = title.chars().take(MAX_TITLE_LEN).collect::<String>();
        title.push_str("...");
    }
    title
}


fn start_com_ports_observer(main_window_weak: Weak<MainWindow>) {
    std::thread::spawn(move || {
        loop {
            let available_ports = available_ports().expect("Can't get available COM ports.")
                .into_iter()
                .filter(|p| p.port_type != SerialPortType::Unknown)
                .map(|p| (&p.port_name).into())
                .collect::<Vec<slint::SharedString>>();

            let _ = main_window_weak.upgrade_in_event_loop(move |w| {
                let current_ports_model = w.get_available_com_ports();
                let all_ports_present = current_ports_model.iter().zip(&available_ports).all(|(c, a)|c==a);
                if current_ports_model.row_count() > 0 && available_ports.len() > 0 && all_ports_present {
                    return
                };

                let selected_port = w.get_selected_com_port();
                let new_selected_port = if selected_port == "" || (selected_port != "" && !available_ports.contains(&selected_port)) {
                    available_ports.first().map(|p| p.clone())
                } else {
                    Some(selected_port)
                };
                
                let model = VecModel::from(available_ports);
                let available_ports_model = Rc::new(model);
                
                w.set_available_com_ports(available_ports_model.into());
                w.set_selected_com_port(new_selected_port.unwrap_or("".into()));
            });
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}


fn start_monitors_observer(main_window_weak: Weak<MainWindow>) {
    let image_bytes = include_bytes!("../monitor-icon.jpg");
    let img: DynamicImage = ImageReader::new(Cursor::new(image_bytes)).with_guessed_format().unwrap().decode().unwrap();
    let pix_buff = SharedPixelBuffer::clone_from_slice(&img.to_rgba8(), img.width(), img.height());
    
    std::thread::spawn(move || {
        let mut handlers = Vec::new();

        loop {
            let monitors: Vec<Monitor> = Monitor::enumerate().unwrap();

            if monitors.len() == handlers.len() && handlers.iter().all(|h: &CaptureControl<Preview, Box<dyn Error + Send + Sync>>| !h.is_finished()) {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }

            for handler in handlers.into_iter() {
                handler.stop().unwrap();
            }

            handlers = Vec::new();
            let pix_buff = pix_buff.clone();
            
            let monitors_copy = monitors.clone();
            let _ = main_window_weak.upgrade_in_event_loop(move |w| {
                let mon_mock = slint::Image::from_rgba8(pix_buff);
                let monitors_vec = monitors_copy.iter().map(|m| {
                    let title = get_monitor_title(m);
                    MonitorData{
                        title: title.into(),
                        image: mon_mock.clone(),
                    }
                }).collect::<Vec<MonitorData>>();
                let monitors_vec_model = Rc::new(VecModel::from(monitors_vec));
                w.set_monitors_data(monitors_vec_model.into())
            });

            for (index, monitor) in monitors.into_iter().enumerate() {
                let handler = start_monitor_preview(index, monitor, main_window_weak.clone());
                handlers.push(handler);
            }
        }
    });
}


fn start_monitor_preview(index: usize, monitor: Monitor, main_window_weak: Weak<MainWindow>) -> CaptureControl<Preview, Box<dyn Error + Send + Sync>> {
    let title = get_monitor_title(&monitor);
    let flags = (title, index, main_window_weak);

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::WithoutBorder,
        ColorFormat::Rgba8,
        flags,
    );

    Preview::start_free_threaded(settings).unwrap()
}


fn main() {
    let main_window = MainWindow::new().unwrap();

    let capture_control: CaptureControlRef = Rc::new(RefCell::new(None));

    start_com_ports_observer(main_window.as_weak());
    start_monitors_observer(main_window.as_weak());
  
    let capture_control_clone = capture_control.clone();
    
    main_window.on_toggle_capturing({
        let main_window_weak = main_window.as_weak();
        move || {
            let main_window = main_window_weak.unwrap();
            let mut capture_lock = capture_control_clone.borrow_mut();
            
            if let Some(capture_control) = capture_lock.take() {
                if !capture_control.is_finished() {
                    capture_control.stop().unwrap();
                    main_window.set_is_capture_running(false);
                    return
                }
            }

            let selected_com_port = main_window.get_selected_com_port();

            let selected_monitor_idx = main_window.get_selected_monitor() as usize;
            let monitor = Monitor::from_index(selected_monitor_idx).unwrap();

            let com_port_builder = new(selected_com_port.to_string(), 250000);
            let mut com_port = com_port_builder.open().unwrap();
            com_port.set_timeout(Duration::from_millis(100)).unwrap();

            let settings = Settings::new(
                monitor,
                CursorCaptureSettings::Default,
                DrawBorderSettings::WithoutBorder,
                ColorFormat::Rgba8,
                (com_port, main_window_weak.clone()),
            );

            main_window.set_is_capture_running(true);
            let capture_control: CaptureControl<Capture, Box<dyn Error + Send + Sync>> = Capture::start_free_threaded(settings).unwrap();
            *capture_lock = Some(capture_control);
        }
    });

    main_window.run().unwrap();

}