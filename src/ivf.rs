use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::io::{Error, ErrorKind};
use super::Bitstream;

pub struct Ivf {
    pub input: String,
    pub output: String,
    pub frame_num: usize,
    pub vp9: bool,
}

impl Ivf {
    pub fn helper() {
        println!("Usage: rsplit ivf input.ivf output frame_num vp8|vp9")
    }

    pub fn new(args: &[String]) -> Result<Ivf, &'static str> {
        let l = args.len() as usize;
        if l < 6 {
            return Err("too less arguments for rsplit ivf mode");
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

        Ok(Ivf {
            input: input,
            output: output,
            frame_num: frame_num,
            vp9: vp9,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        println!("rsplit VP{} {} into {}",
                 8 + (self.vp9 as i32),
                 self.input,
                 self.output);
        let mut fi = try!(File::open(self.input.clone()));

        let mut ivf_seq_buffer = [0u8; 32];
        let bytes_read = fi.read(&mut ivf_seq_buffer).unwrap();
        if bytes_read != 32 {
            return Err(Error::new(ErrorKind::Other, "bytes read is not expected ..."));
        }
        if !(ivf_seq_buffer[0] == 'D' as u8 && ivf_seq_buffer[1] == 'K' as u8 &&
             ivf_seq_buffer[2] == 'I' as u8 && ivf_seq_buffer[3] == 'F' as u8) {
            return Err(Error::new(ErrorKind::Other, "Not supported IVF format ..."));
        }

        //bytes 24-27  number of frames in file
        let mut total_frame_num =
            ((ivf_seq_buffer[27] as u32) << 24) | ((ivf_seq_buffer[26] as u32) << 16) |
            ((ivf_seq_buffer[25] as u32) << 8) | ((ivf_seq_buffer[24] as u32) << 0);
        if total_frame_num == 0 {
            println!("ivf sequence frame num is invalid 0, so set it to default 10000");
            total_frame_num = 10000;
        } else {
            println!("ivf sequence frame num is {}", total_frame_num);
        }

        let mut bs_container: Vec<Bitstream> = Vec::new();
        let mut pre_frame_no = 0;
        for cur_frame_no in 0..(total_frame_num as i32) {
            let bs = match self.find_au_nal_units(&mut fi) {
                Ok(bs) => bs,
                Err(_) => {
                    break;
                }
            };

            if bs.idr_flag {
                print!("IDR");
            } else {
                print!(".");
            }

            if bs.idr_flag && cur_frame_no - pre_frame_no >= (self.frame_num as i32) {
                if let Err(e) = self.write_to_file(&mut pre_frame_no,
                                                   cur_frame_no,
                                                   &mut bs_container,
                                                   &mut ivf_seq_buffer) {
                    return Err(e);
                }
                bs_container.clear();
            }

            bs_container.push(bs);
        }

        if let Err(e) = self.write_to_file(&mut pre_frame_no,
                                           (total_frame_num as i32),
                                           &mut bs_container,
                                           &mut ivf_seq_buffer) {
            return Err(e);
        }

        Ok(())
    }

    fn find_au_nal_units(&self, fp_bs: &mut io::Read) -> io::Result<Bitstream> {
        let mut bs = Bitstream {
            frame_header: vec![0u8; 12],
            frame_location: vec![0u32; 0],
            nal_size: 0,
            frame_data: vec![0u8; 0],
            buf_size: 0,
            idr_flag: false,
        };

        let read_buffer_size = fp_bs.read(&mut bs.frame_header).unwrap();
        if read_buffer_size != 12 {
            return Err(Error::new(ErrorKind::Other, "bytes read 12 is not expected ..."));
        }

        //bytes 0-3    size of frame in bytes (not including the 12-byte header)
        bs.buf_size = ((bs.frame_header[3] as u32) << 24) | ((bs.frame_header[2] as u32) << 16) |
                      ((bs.frame_header[1] as u32) << 8) |
                      ((bs.frame_header[0] as u32) << 0);

        bs.frame_data = vec![0u8; bs.buf_size as usize];
        let read_buffer_size = fp_bs.read(&mut bs.frame_data).unwrap();
        if read_buffer_size != bs.buf_size as usize {
            return Err(Error::new(ErrorKind::Other, "bytes read buf size is not expected ..."));
        }

        if !self.vp9 {
            //VP8
            let key_frame = bs.frame_data[0] & 0x1;
            if key_frame == 0 {
                bs.idr_flag = true;
            } else {
                bs.idr_flag = false;
            }
        } else {
            //VP9
            let show_existing_frame = bs.frame_data[0] & 0x8;
            let key_frame = bs.frame_data[0] & 0x4;
            if key_frame == 0 && show_existing_frame == 0 {
                bs.idr_flag = true;
            } else {
                bs.idr_flag = false;
            }
        }

        Ok(bs)
    }

    fn write_to_file(&self,
                     pre_frame_no: &mut i32,
                     cur_frame_no: i32,
                     bs_container: &mut [Bitstream],
                     ivf_seq_buffer: &mut [u8])
                     -> io::Result<()> {
        let output_ivf = format!("{}_{:04}_{:04}.ivf",
                                 self.output,
                                 *pre_frame_no,
                                 cur_frame_no - 1);

        println!("\nFrames[{:04}-{:04}] => {}\n",
                 *pre_frame_no,
                 cur_frame_no - 1,
                 output_ivf);

        let mut fo = try!(File::create(output_ivf));

        let frame_num = cur_frame_no - *pre_frame_no;
        ivf_seq_buffer[24] = ((frame_num >> 0) & 0xFF) as u8;
        ivf_seq_buffer[25] = ((frame_num >> 8) & 0xFF) as u8;
        ivf_seq_buffer[26] = ((frame_num >> 16) & 0xFF) as u8;
        ivf_seq_buffer[27] = ((frame_num >> 24) & 0xFF) as u8;

        let bytes_write = fo.write(&ivf_seq_buffer).unwrap();
        if bytes_write != 32 {
            return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
        }

        for b in bs_container {
            let bytes_write = fo.write(&b.frame_header).unwrap();
            if bytes_write != b.frame_header.len() {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
            let bytes_write = fo.write(&b.frame_data).unwrap();
            if bytes_write != b.frame_data.len() {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
        }

        *pre_frame_no = cur_frame_no;

        Ok(())
    }
}
