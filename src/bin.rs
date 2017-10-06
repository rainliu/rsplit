use std::fs::File;
use std::io;
use std::io::{Read, Write, Seek, SeekFrom};
use std::io::{Error, ErrorKind};
use super::Bitstream;

pub struct Bin {
    pub input: String,
    pub output: String,
    pub frame_num: usize,
    pub h265: bool,
}

impl Bin {
    pub fn helper() {
        println!("Usage: rsplit bin input.bin output frame_num h264|h265")
    }

    pub fn new(args: &[String]) -> Result<Bin, &'static str> {
        let l = args.len() as usize;
        if l < 6 {
            return Err("too less arguments for rsplit bin mode");
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
        let h265 = match args[5].to_lowercase().as_ref() {
            "h265" => true,
            "h264" => false,
            _ => {
                return Err("only support h264 and h265");
            }
        };

        Ok(Bin {
            input: input,
            output: output,
            frame_num: frame_num,
            h265: h265,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        println!("rsplit H26{} {} into {}",
                 4 + (self.h265 as i32),
                 self.input,
                 self.output);
        let mut fi = try!(File::open(self.input.clone()));

        let mut bk_container: Vec<u8> = Vec::new();
        let mut bs_container: Vec<Bitstream> = Vec::new();
        let mut pre_frame_no = 0;
        let mut cur_frame_no = 0;
        let mut bak_byte_pos = 0;
        loop {
            let (eof, opt) = if self.h265 {
                self.find_h265_nal_units(&mut fi)
            }else{
                self.find_h264_nal_units(&mut fi)
            };

            let mut bs = match opt {
                Ok(bs) => bs,
                Err(_) => {
                    break;
                }
            };
            bs.buf_size = bs.frame_location[bs.nal_size] - bs.frame_location[0];

            if bs.idr_flag {
                print!("IDR");
            } else {
                print!(".");
            }

            if bs.idr_flag && cur_frame_no - pre_frame_no >= (self.frame_num as i32) {
                if let Err(e) = self.write_to_file(&mut pre_frame_no,
                                                   cur_frame_no,
                                                   bak_byte_pos,
                                                   &mut bk_container,
                                                   &mut bs_container) {
                    return Err(e);
                }
                bs_container.clear();
                bak_byte_pos = bk_container.len();
            }

            if self.h265 {
                for i in 0..bs.nal_size {
                    if bs.frame_header[i]==32 || /*NAL_UNIT_VPS*/ bs.frame_header[i]==33 || /*NAL_UNIT_SPS*/ bs.frame_header[i]==34 /*NAL_UNIT_PPS*/
                    {
                        for j in 0..bs.frame_location[i + 1] - bs.frame_location[i] {
                            bk_container.push(bs.frame_data[(bs.frame_location[i] + j) as usize]);
                        }
                    }
                }
            }else{
                for i in 0..bs.nal_size {
                    if bs.frame_header[i]==7 || /*NAL_UNIT_SPS*/ bs.frame_header[i]==8 /*NAL_UNIT_PPS*/
                    {
                        for j in 0..bs.frame_location[i + 1] - bs.frame_location[i] {
                            bk_container.push(bs.frame_data[(bs.frame_location[i] + j) as usize]);
                        }
                    }
                }
            }

            bs_container.push(bs);

            if eof {
                break;
            }

            cur_frame_no += 1;
        }

        if let Err(e) = self.write_to_file(&mut pre_frame_no,
                                           cur_frame_no,
                                           bak_byte_pos,
                                           &mut bk_container,
                                           &mut bs_container) {
            return Err(e);
        }

        Ok(())
    }

    fn find_h265_nal_units(&self, fp_bs: &mut File) -> (bool, io::Result<Bitstream>) {
        const START_CODE_SIZE: i32 = 3;
        const MAX_NAL_UNITS_PER_BS: usize = 600;

        let mut bs = Bitstream {
            frame_header: vec![0u8; MAX_NAL_UNITS_PER_BS+1],
            frame_location: vec![0u32; MAX_NAL_UNITS_PER_BS+1],
            nal_size: 0,
            frame_data: vec![0u8; 0],
            buf_size: 0,
            idr_flag: false,
        };

        let mut buf = [0u8; 1];
        let mut frame_data_size = 0;
        let mut num_nal_units = 0;
        let mut zeros = 0;
        let mut pic_found_flag = false;
        let mut bs_size_since_last_slice = START_CODE_SIZE;
        let mut num_nal_units_since_last_slice = 0;
        let mut last_slice_flag = true;

        loop {
            match fp_bs.read(&mut buf) {
                Ok(n) => {
                    if n != 1 {
                        bs.frame_location[num_nal_units] = frame_data_size;
                        bs.nal_size = num_nal_units;
                        return (true, Ok(bs));
                    }
                }
                Err(e) => {
                    return (false, Err(e));
                }
            };
            bs.frame_data.push(buf[0]);
            frame_data_size += 1;
            if !last_slice_flag {
                bs_size_since_last_slice += 1;
            }

            match buf[0] {
                0 => {
                    zeros += 1;
                }

                1 => {
                    if zeros > 1 {
                        // find trailing_zero_8bits and 0x000001
                        bs.frame_location[num_nal_units] = frame_data_size - zeros - 1;

                        match fp_bs.read(&mut buf) {
                            Ok(n) => {
                                if n != 1 {
                                    return (true, Ok(bs));
                                }
                            }
                            Err(e) => {
                                return (false, Err(e));
                            }
                        };

                        let nal_unit_type = (buf[0] & 0x7E) >> 1;

                        bs.frame_header[num_nal_units] = nal_unit_type;

                        if nal_unit_type <= 23 {
                            // SLICE FOUND
                            match fp_bs.read(&mut buf) {
                                Ok(n) => {
                                    if n != 1 {
                                        return (true, Ok(bs));
                                    }
                                }
                                Err(e) => {
                                    return (false, Err(e));
                                }
                            };
                            match fp_bs.read(&mut buf) {
                                Ok(n) => {
                                    if n != 1 {
                                        return (true, Ok(bs));
                                    }
                                }
                                Err(e) => {
                                    return (false, Err(e));
                                }
                            };

                            let first_slice_in_pic_flag = (buf[0] >> 7) != 0;

                            if first_slice_in_pic_flag {
                                if pic_found_flag {
                                    fp_bs.seek(SeekFrom::Current(-3 -
                                                                (bs_size_since_last_slice as i64)))
                                        .unwrap();
                                    bs.nal_size = num_nal_units - num_nal_units_since_last_slice;

                                    return (false, Ok(bs));
                                } else {
                                    fp_bs.seek(SeekFrom::Current(-3)).unwrap();
                                    num_nal_units += 1;
                                    zeros = 0;
                                    pic_found_flag = true;
                                    bs.idr_flag = nal_unit_type==19 /*NAL_UNIT_CODED_SLICE_IDR*/ || nal_unit_type == 20 /*NAL_UNIT_CODED_SLICE_IDR_N_LP*/;
                                }
                            } else {
                                fp_bs.seek(SeekFrom::Current(-3)).unwrap();
                                num_nal_units += 1;
                                zeros = 0;
                            }

                            last_slice_flag = true;
                            bs_size_since_last_slice = START_CODE_SIZE;
                            num_nal_units_since_last_slice = 0;
                        } else {
                            if nal_unit_type == 40 {
                                //Suffix SEI
                                last_slice_flag = true;
                                bs_size_since_last_slice = START_CODE_SIZE;
                                num_nal_units_since_last_slice = 0;
                            } else {
                                last_slice_flag = false;
                                num_nal_units_since_last_slice += 1;
                            }

                            fp_bs.seek(SeekFrom::Current(-1)).unwrap();
                            num_nal_units += 1;
                            zeros = 0;
                        }
                    } else {
                        zeros = 0;
                    }
                }

                _ => {
                    zeros = 0;
                }
            }
        }
    }

    fn find_h264_nal_units(&self, fp_bs: &mut File) -> (bool, io::Result<Bitstream>) {
        const START_CODE_SIZE: i32 = 3;
        const MAX_NAL_UNITS_PER_BS: usize = 600;

        let mut bs = Bitstream {
            frame_header: vec![0u8; MAX_NAL_UNITS_PER_BS+1],
            frame_location: vec![0u32; MAX_NAL_UNITS_PER_BS+1],
            nal_size: 0,
            frame_data: vec![0u8; 0],
            buf_size: 0,
            idr_flag: false,
        };

        let mut buf = [0u8; 1];
        let mut frame_data_size = 0;
        let mut num_nal_units = 0;
        let mut zeros = 0;
        let mut pic_found_flag = false;
        let mut bs_size_since_last_slice = START_CODE_SIZE;
        let mut num_nal_units_since_last_slice = 0;
        let mut last_slice_flag = true;

        loop {
            match fp_bs.read(&mut buf) {
                Ok(n) => {
                    if n != 1 {
                        bs.frame_location[num_nal_units] = frame_data_size;
                        bs.nal_size = num_nal_units;
                        return (true, Ok(bs));
                    }
                }
                Err(e) => {
                    return (false, Err(e));
                }
            };
            bs.frame_data.push(buf[0]);
            frame_data_size += 1;
            if !last_slice_flag {
                bs_size_since_last_slice += 1;
            }

            match buf[0] {
                0 => {
                    zeros += 1;
                }

                1 => {
                    if zeros > 1 {
                        // find trailing_zero_8bits and 0x000001
                        bs.frame_location[num_nal_units] = frame_data_size - zeros - 1;

                        match fp_bs.read(&mut buf) {
                            Ok(n) => {
                                if n != 1 {
                                    return (true, Ok(bs));
                                }
                            }
                            Err(e) => {
                                return (false, Err(e));
                            }
                        };

                        let nal_unit_type = buf[0] & 0x1F;

                        bs.frame_header[num_nal_units] = nal_unit_type;

                        if nal_unit_type <= 5 {
                            // SLICE FOUND
                            if nal_unit_type == 1 || nal_unit_type == 2  ||nal_unit_type == 5 {
                                match fp_bs.read(&mut buf) {
                                    Ok(n) => {
                                        if n != 1 {
                                            return (true, Ok(bs));
                                        }
                                    }
                                    Err(e) => {
                                        return (false, Err(e));
                                    }
                                };

                                let first_slice_in_pic_flag = (buf[0] >> 7) != 0;

                                if first_slice_in_pic_flag {
                                    if pic_found_flag {
                                        fp_bs.seek(SeekFrom::Current(-2 - (bs_size_since_last_slice as i64))).unwrap();
                                        bs.nal_size = num_nal_units - num_nal_units_since_last_slice;

                                        return (false, Ok(bs));
                                    } else {
                                        fp_bs.seek(SeekFrom::Current(-2)).unwrap();
                                        num_nal_units += 1;
                                        zeros = 0;
                                        pic_found_flag = true;
                                        bs.idr_flag = nal_unit_type==5;
                                    }
                                } else {
                                    fp_bs.seek(SeekFrom::Current(-2)).unwrap();
                                    num_nal_units += 1;
                                    zeros = 0;
                                }
                            }else{
                                fp_bs.seek(SeekFrom::Current(-1)).unwrap();
                                num_nal_units += 1;
                                zeros = 0;    
                            }

                            last_slice_flag = true;
                            bs_size_since_last_slice = START_CODE_SIZE;
                            num_nal_units_since_last_slice = 0;
                        } else {
                            last_slice_flag = false;
                            num_nal_units_since_last_slice += 1;
                            
                            fp_bs.seek(SeekFrom::Current(-1)).unwrap();
                            num_nal_units += 1;
                            zeros = 0;
                        }
                    } else {
                        zeros = 0;
                    }
                }

                _ => {
                    zeros = 0;
                }
            }
        }
    }

    fn write_to_file(&self,
                     pre_frame_no: &mut i32,
                     cur_frame_no: i32,
                     bak_byte_pos: usize,
                     bk_container: &mut [u8],
                     bs_container: &mut [Bitstream])
                     -> io::Result<()> {
        let output_bin = format!("{}_{:04}_{:04}.bin",
                                 self.output,
                                 *pre_frame_no,
                                 cur_frame_no - 1);

        println!("\nFrames[{:04}-{:04}] => {}\n",
                 *pre_frame_no,
                 cur_frame_no - 1,
                 output_bin);

        let mut fo = try!(File::create(output_bin));
        {
            let bytes_write = fo.write(&bk_container[0..bak_byte_pos]).unwrap();
            if bytes_write != bak_byte_pos {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
        }
        for b in bs_container {
            let bytes_write = fo.write(&b.frame_data[0..(b.buf_size as usize)]).unwrap();
            if bytes_write != (b.buf_size as usize) {
                return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
            }
        }

        *pre_frame_no = cur_frame_no;

        Ok(())
    }
}
