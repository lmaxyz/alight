// use std::time::SystemTime;

use serialport::SerialPort;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl
};
use rayon::prelude::*;


const LEDS_WIDTH: u32 = 28;
const LEDS_HEIGHT: u32 = 20;


// Struct To Implement The Trait For
pub struct Capture {
    com_port: Box<dyn SerialPort>,

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

    fn calc_target_pixel(source_index: u32, subrow_len: u32, subcol_len: u32, row_pitch: u32, target_col_index: u32, raw_buffer: &[u8]) -> [u8; 3] {
        let mut target_pixel_sum = (0,0,0,0);
        let target_col_index_offset = target_col_index * 4;
        for subrow in 0..subrow_len {
            let start_index = source_index + (subrow * row_pitch) + (target_col_index_offset * subcol_len);
            let end_index = start_index + subcol_len * 4;
            let subrow_pixel_sum = raw_buffer[start_index as usize .. end_index as usize].chunks(4).fold((0,0,0),|mut acc:(u32,u32,u32), x| {
                acc.0 += x[0] as u32;
                acc.1 += x[1] as u32;
                acc.2 += x[2] as u32;
                acc
            });

            target_pixel_sum.0 += subrow_pixel_sum.0 / subcol_len;
            target_pixel_sum.1 += subrow_pixel_sum.1 / subcol_len;
            target_pixel_sum.2 += subrow_pixel_sum.2 / subcol_len;
        }
        [
            (target_pixel_sum.0 / subrow_len) as u8,
            (target_pixel_sum.1 / subrow_len) as u8,
            (target_pixel_sum.2 / subrow_len) as u8
        ]
    }
}

impl GraphicsCaptureApiHandler for Capture {
    // To Get The Message From The Settings
    type Flags = Box<dyn SerialPort>;
    // type Flags = String;

    // To Redirect To CaptureControl Or Start Method
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function That Will Be Called To Create The Struct The Flags Can Be Passed
    // From `WindowsCaptureSettings`
    fn new(com_port: Self::Flags) -> Result<Self, Self::Error> {
        let left: Vec<u8> = Vec::with_capacity((LEDS_HEIGHT*3) as usize);
        let right: Vec<u8> = Vec::with_capacity((LEDS_HEIGHT*3) as usize);
        let top: Vec<u8> = Vec::with_capacity((LEDS_WIDTH*3) as usize);
        let bottom: Vec<u8> = Vec::with_capacity((LEDS_WIDTH*3) as usize);

        Ok(Self {
            com_port,
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
        // let time_start = SystemTime::now();
        
        let mut frame_buffer = frame.buffer().unwrap();
        let row_pitch = frame_buffer.row_pitch();
        let subcol_len = frame_buffer.width() / LEDS_WIDTH;
        let subrow_len = frame_buffer.height() / LEDS_HEIGHT;

        let raw_buffer = frame_buffer.as_raw_buffer();

        let source_top_index = 0 * subrow_len * row_pitch;
        let source_bottom_index = (LEDS_HEIGHT-1) * subrow_len * row_pitch;

        let width_results: Vec<([u8; 3], [u8; 3])> = (0..LEDS_WIDTH).into_par_iter().map(|target_col_index| {
            // calc top
            let top_pixel = Capture::calc_target_pixel(source_top_index, subrow_len, subcol_len, row_pitch, target_col_index, &raw_buffer);
            // calc bottom
            let bottom_pixel = Capture::calc_target_pixel(source_bottom_index, subrow_len, subcol_len, row_pitch, target_col_index, &raw_buffer);

            (top_pixel, bottom_pixel)
        }).collect();

        for (top_pixel, bottom_pixel) in width_results.into_iter() {
            self.top.extend_from_slice(&top_pixel);
            self.bottom.extend_from_slice(&bottom_pixel);
        }

        let height_results: Vec<([u8; 3], [u8; 3])> = (1..LEDS_HEIGHT).into_par_iter().map(|target_row_index| {
            let source_index = target_row_index * subrow_len * row_pitch;

            // calc right
            let right_pixel = Capture::calc_target_pixel(source_index, subrow_len, subcol_len, row_pitch, LEDS_WIDTH-1, &raw_buffer);
            // calc left
            let left_pixel = Capture::calc_target_pixel(source_index, subrow_len, subcol_len, row_pitch, 0, &raw_buffer);

            (left_pixel, right_pixel)
        }).collect();

        for (left_pixel, right_pixel) in height_results.into_iter() {
            self.left.extend_from_slice(&left_pixel);
            self.right.extend_from_slice(&right_pixel);
        }

        // Reverse this sides to correct displaying
        let top:Vec<u8>  = self.top.chunks(3).rev().flatten().map(|x| *x).collect();
        let right:Vec<u8>  = self.right.chunks(3).rev().flatten().map(|x| *x).collect();
        self.com_port.write(b"Ada").unwrap();
        self.com_port.write(self.bottom.as_slice()).unwrap();
        self.com_port.write(right.as_slice()).unwrap();
        self.com_port.write(top.as_slice()).unwrap();
        self.com_port.write(self.left.as_slice()).unwrap();

        // println!("Elapsed: {}", time_start.elapsed().unwrap().as_secs_f32());

        // Gracefully Stop The Capture Thread
        // if self.frames_counter >= 60 {
        //     capture_control.stop();
        //     println!("{}", (self.total_time_elapsed / 60.0))
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
        println!("Capture Session Closed");

        Ok(())
    }
}