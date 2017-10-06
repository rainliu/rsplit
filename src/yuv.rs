use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::io::{Error, ErrorKind};

pub struct Yuv {
    pub input_yuv: String,
    pub output_prefix: String,
    pub frame_num: usize,
    pub frame_size: Vec<(i32, i32)>,
}

impl Yuv {
    pub fn helper() {
        println!("Usage: rsplit yuv input.yuv output_prefix frame_num frame_size1 \
                  [...|frame_size2 ...]")
    }

    pub fn new(args: &[String]) -> Result<Yuv, &'static str> {
        let l = args.len() as usize;
        if l < 6 {
            return Err("too less arguments for rsplit yuv mode");
        }

        let input_yuv = args[2].clone();
        let output_prefix = args[3].clone();
        let frame_num_opt = args[4].clone().parse::<usize>();
        let frame_num = match frame_num_opt {
            Ok(frame_num) => frame_num,
            Err(_) => {
                return Err("can't parse frame_num as usize");
            }
        };

        let mut frame_size: Vec<(i32, i32)> = Vec::new();
        for i in 0..frame_num {
            if 5 + i >= l {
                return Err("too less arguments for rsplit yuv mode");
            } else if args[5 + i] == "..." {
                if i == 0 {
                    return Err("... can't be the first frame size");
                } else {
                    let (width, height) = frame_size[i - 1];
                    for _ in i..frame_num {
                        frame_size.push((width, height));
                    }
                    break;
                }
            } else {
                let frame_size_str = args[5 + i].clone();
                let nums: Vec<&str> = frame_size_str.split("x").collect();
                if nums.len() != 2 {
                    return Err("invalid frame size");
                }
                if let Ok(width) = nums[0].parse::<i32>() {
                    if let Ok(height) = nums[1].parse::<i32>() {
                        frame_size.push((width, height));
                    } else {
                        return Err("invalid frame height");
                    }
                } else {
                    return Err("invalid frame width");
                }
            }
        }

        Ok(Yuv {
            input_yuv: input_yuv,
            output_prefix: output_prefix,
            frame_num: frame_num,
            frame_size: frame_size,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        println!("rsplit {} into {}", self.input_yuv, self.output_prefix);
        let mut fi = try!(File::open(self.input_yuv.clone()));
        const BUF_SIZE: usize = 2048;
        let mut buf = [0u8; BUF_SIZE];

        for i in 0..self.frame_num {
            let output_yuv = self.output_prefix.clone() + "_" + &i.to_string() + "_" +
                             &self.frame_size[i].0.to_string() +
                             "x" + &self.frame_size[i].1.to_string() +
                             ".yuv";
            println!("Frame {} - {}x{} in {} ...",
                     i,
                     self.frame_size[i].0,
                     self.frame_size[i].1,
                     output_yuv);
            let mut buf_size = (self.frame_size[i].0 * self.frame_size[i].1 +
                                ((self.frame_size[i].0 + 1) / 2) *
                                ((self.frame_size[i].1 + 1) / 2) *
                                2) as usize;
            let mut fo = try!(File::create(output_yuv));
            while buf_size > 0 {
                let bytes = if buf_size > BUF_SIZE {
                    BUF_SIZE
                } else {
                    buf_size
                };
                buf_size -= bytes;
                let bytes_read = fi.read(&mut buf[0..bytes]).unwrap();
                if bytes_read != bytes {
                    return Err(Error::new(ErrorKind::Other, "bytes read is not expected ..."));
                }
                let bytes_write = fo.write(&buf[0..bytes]).unwrap();
                if bytes_write != bytes {
                    return Err(Error::new(ErrorKind::Other, "bytes write is not expected ..."));
                }
            }
        }

        Ok(())
    }
}
