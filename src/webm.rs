use std::fs::File;
use std::io;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::ffi::CString;
use std::os::raw::c_void;
use std::slice;

#[link(name = "nestegg")]
extern "C" {
    fn vpx_init(filename: *const i8) -> *mut c_void;
    fn vpx_read(input: *mut c_void, length: *mut u32) -> *const u8;
    fn vpx_destroy(input: *mut c_void);
}

pub struct Webm {
    pub input: String,
    pub output: String,
    pub frame_num: usize,
    pub vp9: bool,
}

impl Webm {
    pub fn helper() {
        println!("Usage: rsplit webm input.webm output.ivf frame_num vp8|vp9")
    }

    pub fn new(args: &[String]) -> Result<Webm, &'static str> {
        let l = args.len() as usize;
        if l < 6 {
            return Err("too less arguments for rsplit webm mode");
        }

        let input = args[2].clone();
        let output = args[3].clone();
        let frame_num_opt = args[4].clone().parse::<usize>();
        let frame_num = match frame_num_opt {
            Ok(frame_num) => frame_num,
            Err(_) => {
                return Err("can't parse frame_num as usize");
            }
        };
        let vp9 = match args[5].to_lowercase().as_ref() {
            "vp9" => true,
            "vp8" => false,
            _ => {
                return Err("only support vp8 and vp9");
            }
        };

        Ok(Webm {
            input: input,
            output: output,
            frame_num: frame_num,
            vp9: vp9,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        println!("Convert {} into {}", self.input, self.output);
        let c_input_string =
            unsafe { CString::from_vec_unchecked(self.input.clone().into_bytes()) };
        let input_ctx = unsafe { vpx_init(c_input_string.as_ptr()) };

        let mut fo = try!(File::create(self.output.clone()));
        let mut ivf_seq_header = [0u8; 32];
        let mut ivf_frame_header = [0u8; 12];

        ivf_seq_header[0] = 'D' as u8;
        ivf_seq_header[1] = 'K' as u8;
        ivf_seq_header[2] = 'I' as u8;
        ivf_seq_header[3] = 'F' as u8;
        ivf_seq_header[4] = 0; //version[0]
        ivf_seq_header[5] = 0; //version[1]
        ivf_seq_header[6] = 32; //length[0]
        ivf_seq_header[7] = 0; //length[1]
        ivf_seq_header[8] = 'V' as u8; //fourcc[0]
        ivf_seq_header[9] = 'P' as u8; //fourcc[1]
        if self.vp9 {
            ivf_seq_header[10] = '9' as u8; //fourcc[2]
        } else {
            ivf_seq_header[10] = '8' as u8; //fourcc[2]
        }
        ivf_seq_header[11] = '0' as u8; //fourcc[3]
        ivf_seq_header[12] = 0; //width[0]
        ivf_seq_header[13] = 0; //width[1]
        ivf_seq_header[14] = 0; //height[0]
        ivf_seq_header[15] = 0; //height[1]
        ivf_seq_header[16] = 0xE8; //frame_rate[0]
        ivf_seq_header[17] = 0x03; //frame_rate[1]
        ivf_seq_header[18] = 0x00; //frame_rate[2]
        ivf_seq_header[19] = 0x00; //frame_rate[3]
        ivf_seq_header[20] = 0x01; //time_scale[0]
        ivf_seq_header[21] = 0x00; //time_scale[1]
        ivf_seq_header[22] = 0x00; //time_scale[2]
        ivf_seq_header[23] = 0x00; //time_scale[3]
        ivf_seq_header[24] = ((self.frame_num >> 0) & 0xFF) as u8; //framenum[0]
        ivf_seq_header[25] = ((self.frame_num >> 8) & 0xFF) as u8; //framenum[1]
        ivf_seq_header[26] = ((self.frame_num >> 16) & 0xFF) as u8; //framenum[2]
        ivf_seq_header[27] = ((self.frame_num >> 24) & 0xFF) as u8; //framenum[3]
        ivf_seq_header[28] = 0; //unused[0]
        ivf_seq_header[29] = 0; //unused[1]
        ivf_seq_header[30] = 0; //unused[2]
        ivf_seq_header[31] = 0; //unused[3]

        let bytes_write = fo.write(&ivf_seq_header).unwrap();
        if bytes_write != 32 {
            return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
        }

        let mut frame_no = 0;
        while frame_no < self.frame_num {
            let mut len = 0;
            let ptr = unsafe { vpx_read(input_ctx, &mut len) };
            let buffer = unsafe { slice::from_raw_parts(ptr, len as usize) };

            if len != 0 {
                println!("Frame {:04}: {:8} bytes", frame_no, len);
            } else {
                break;
            }
            frame_no += 1;

            ivf_frame_header[0] = ((len >> 0) & 0xFF) as u8; //frameSize[0]
            ivf_frame_header[1] = ((len >> 8) & 0xFF) as u8; //frameSize[1]
            ivf_frame_header[2] = ((len >> 16) & 0xFF) as u8; //frameSize[2]
            ivf_frame_header[3] = ((len >> 24) & 0xFF) as u8; //frameSize[3]
            let bytes_write = fo.write(&ivf_frame_header).unwrap();
            if bytes_write != 12 {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
            let bytes_write = fo.write(&buffer[0..(len as usize)]).unwrap();
            if bytes_write != (len as usize) {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
        }

        unsafe { vpx_destroy(input_ctx) };

        Ok(())
    }
}
