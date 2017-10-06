pub mod bin;
pub mod ivf;
pub mod psnr;
pub mod webm;
pub mod yuv;

pub struct Bitstream {
    frame_header: Vec<u8>,
    frame_location: Vec<u32>,
    nal_size: usize,
    frame_data: Vec<u8>,
    buf_size: u32,
    idr_flag: bool,
}
