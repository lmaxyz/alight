use std::{thread, time::{Duration, SystemTime}};
use tracing::debug;
use serialport::SerialPort;
use slint::Weak;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl
};
use rayon::prelude::*;

use crate::MainWindow;


const HORIZONTAL_LEDS_NUM: u32 = 28;
const VERTICAL_LEDS_NUM: u32 = 18;

const HORIZONTAL_MULTIPLIER: u32 = 2;
const VERTICAL_MULTIPLIER: u32 = 2;

// ToDo: Use it to pass config from UI
pub struct _CaptureSettings {
    pub horizontal_leds_num: u32,
    pub vertical_leds_num: u32,
    pub grab_horizontal_offset: u32,
    pub grab_vertical_offset: u32
}

fn make_pixel_colorful(pixel: &mut [u8]) {
    let pix_part = pixel.iter_mut().max().unwrap();
    if *pix_part > 50 && *pix_part <= 200 { *pix_part += 40 };
}


// Struct To Implement The Trait For
pub struct Capture {
    com_port: Box<dyn SerialPort>,
    main_window: Weak<MainWindow>,

    left: Vec<u8>,
    right: Vec<u8>,
    top: Vec<u8>,
    bottom: Vec<u8>,

    // frames_counter: u8,
    // total_time_elapsed: f32
}

impl Capture {
    fn release_sides_buffers(&mut self) {
        self.left.clear();
        self.right.clear();
        self.top.clear();
        self.bottom.clear();
    }


    fn calc_target_pixel(source_index: u32, subrow_len: u32, subcol_len: u32, row_pitch: u32, target_col_index: u32, horizontal_multiplier: u32, vertical_multiplier: u32, raw_buffer: &[u8]) -> [u8; 3] {
        let mut target_pixel_sum = (0,0,0,0);
        let subcol_calc_len = subcol_len * horizontal_multiplier * 4;
        let target_col_index_offset = target_col_index * 4 * subcol_len;
        
        for subrow in 0..subrow_len*vertical_multiplier {
            let start_index = source_index + (subrow * row_pitch) + target_col_index_offset;
            let end_index = start_index + subcol_calc_len;
            let subrow_pixel_sum = raw_buffer[start_index as usize .. end_index as usize].chunks(4).fold((0,0,0),|mut acc:(u32,u32,u32), x| {
                acc.0 += x[0] as u32;
                acc.1 += x[1] as u32;
                acc.2 += x[2] as u32;
                acc
            });

            target_pixel_sum.0 += subrow_pixel_sum.0 / (subcol_len*horizontal_multiplier);
            target_pixel_sum.2 += subrow_pixel_sum.2 / (subcol_len*horizontal_multiplier);
            target_pixel_sum.1 += subrow_pixel_sum.1 / (subcol_len*horizontal_multiplier);
        }
        [
            (target_pixel_sum.0 / (subrow_len*vertical_multiplier)) as u8,
            (target_pixel_sum.1 / (subrow_len*vertical_multiplier)) as u8,
            (target_pixel_sum.2 / (subrow_len*vertical_multiplier)) as u8
        ]
    }
}

impl GraphicsCaptureApiHandler for Capture {
    // To Get The Message From The Settings
    type Flags = (Box<dyn SerialPort>, Weak<MainWindow>);
    // type Flags = String;

    // To Redirect To CaptureControl Or Start Method
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function That Will Be Called To Create The Struct The Flags Can Be Passed
    // From `WindowsCaptureSettings`
    fn new((com_port, main_window): Self::Flags) -> Result<Self, Self::Error> {
        let left: Vec<u8> = Vec::with_capacity((VERTICAL_LEDS_NUM*3) as usize);
        let right: Vec<u8> = Vec::with_capacity((VERTICAL_LEDS_NUM*3) as usize);
        let top: Vec<u8> = Vec::with_capacity((HORIZONTAL_LEDS_NUM*3) as usize);
        let bottom: Vec<u8> = Vec::with_capacity((HORIZONTAL_LEDS_NUM*3) as usize);

        Ok(Self {
            com_port,
            main_window,
            left,
            right,
            top, 
            bottom,
            // frames_counter:0,
            // total_time_elapsed: 0.0
        })
    }

    // Called Every Time A New Frame Is Available
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Check performance
        let time_start = SystemTime::now();

        let mut frame_buffer = frame.buffer().unwrap();
        let row_pitch = frame_buffer.row_pitch();
        let subcol_width = frame_buffer.width() / HORIZONTAL_LEDS_NUM;
        let subrow_height = frame_buffer.height() / VERTICAL_LEDS_NUM;

        let raw_buffer = frame_buffer.as_raw_buffer();

        let source_top_index = 0 * subrow_height * row_pitch;
        let source_bottom_index = (VERTICAL_LEDS_NUM-VERTICAL_MULTIPLIER) * subrow_height * row_pitch;

        let width_results: Vec<([u8; 3], [u8; 3])> = (0..HORIZONTAL_LEDS_NUM).into_par_iter().map(|target_col_index| {
            // calc top
            let mut top_pixel = Capture::calc_target_pixel(source_top_index, subrow_height, subcol_width, row_pitch, target_col_index, 1, VERTICAL_MULTIPLIER, &raw_buffer);
            make_pixel_colorful(&mut top_pixel);

            // calc bottom
            let mut bottom_pixel = Capture::calc_target_pixel(source_bottom_index, subrow_height, subcol_width, row_pitch, target_col_index-VERTICAL_MULTIPLIER, 1, VERTICAL_MULTIPLIER, &raw_buffer);
            make_pixel_colorful(&mut bottom_pixel);

            (top_pixel, bottom_pixel)
        }).collect();

        for (top_pixel, bottom_pixel) in width_results.into_iter() {
            self.top.extend_from_slice(&top_pixel);
            self.bottom.extend_from_slice(&bottom_pixel);
        }

        let height_results: Vec<([u8; 3], [u8; 3])> = (0..VERTICAL_LEDS_NUM).into_par_iter().map(|target_row_index| {
            let source_index = target_row_index * subrow_height * row_pitch;

            // calc right
            let mut right_pixel = Capture::calc_target_pixel(source_index, subrow_height, subcol_width, row_pitch, HORIZONTAL_LEDS_NUM-HORIZONTAL_MULTIPLIER, HORIZONTAL_MULTIPLIER, 1, &raw_buffer);
            make_pixel_colorful(&mut right_pixel);
            
            // calc left
            let mut left_pixel = Capture::calc_target_pixel(source_index, subrow_height, subcol_width, row_pitch, 0, HORIZONTAL_MULTIPLIER, 1, &raw_buffer);
            make_pixel_colorful(&mut left_pixel);
            

            (left_pixel, right_pixel)
        }).collect();

        for (left_pixel, right_pixel) in height_results.into_iter() {
            self.left.extend_from_slice(&left_pixel);
            self.right.extend_from_slice(&right_pixel);
        }

        // Reverse this sides to correct displaying
        self.top = self.top.chunks(3).rev().flatten().map(|x| *x).collect();
        self.right  = self.right.chunks(3).rev().flatten().map(|x| *x).collect();
        self.com_port.write(b"Ada").unwrap();
        self.com_port.write(self.bottom.as_slice()).unwrap();
        self.com_port.write(self.right.as_slice()).unwrap();
        self.com_port.write(self.top.as_slice()).unwrap();
        self.com_port.write(self.left.as_slice()).unwrap();

        // Holds 60 FPS
        if let Ok(elapsed) = time_start.elapsed() {
            let wait_time = 16666 - elapsed.as_micros();
            if elapsed.as_micros() < 16666 {
                thread::sleep(Duration::from_micros(wait_time as u64));
            }
        }

        // if self.total_time_elapsed >= 1.0 {
        //     debug!("FPS: {}", self.frames_counter);
        //     self.frames_counter = 0;
        //     self.total_time_elapsed = 0.0;
        // } else {
        //     self.frames_counter += 1;
        //     self.total_time_elapsed += time_start.elapsed().unwrap().as_secs_f32();
        // }
        self.release_sides_buffers();
        
        Ok(())
    }

    // Called When The Capture Item Closes Usually When The Window Closes, Capture
    // Session Will End After This Function Ends
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        debug!("Capture Session Closed");
        self.main_window.upgrade_in_event_loop(|w| { w.set_is_capture_running(false) }).unwrap();
        Ok(())
    }
}