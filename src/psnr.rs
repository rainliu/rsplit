use std::fs::File;
use std::io;
use std::io::Read;
use std::io::{Error, ErrorKind};

pub struct Psnr {
    pub input1_yuv: String,
    pub input2_yuv: String,
    pub frame_num: usize,
    pub frame_size: Vec<(i32, i32)>,
}

impl Psnr {
    pub fn helper() {
        println!("Usage: rsplit psnr input1.yuv input2.yuv frame_num frame_size1 [...|frame_size2 \
                  ...]")
    }

    pub fn new(args: &[String]) -> Result<Psnr, &'static str> {
        let l = args.len() as usize;
        if l < 6 {
            return Err("too less arguments for rsplit psnr mode");
        }

        let input1_yuv = args[2].clone();
        let input2_yuv = args[3].clone();
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
                return Err("too less arguments for rsplit psnr mode");
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

        Ok(Psnr {
            input1_yuv: input1_yuv,
            input2_yuv: input2_yuv,
            frame_num: frame_num,
            frame_size: frame_size,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        println!("psnr {} vs {}", self.input1_yuv, self.input2_yuv);
        let mut f1 = try!(File::open(self.input1_yuv.clone()));
        let mut f2 = try!(File::open(self.input2_yuv.clone()));

        let mut total_psnr_y = 0.0;
        let mut total_psnr_u = 0.0;
        let mut total_psnr_v = 0.0;
        let mut total_psnr = 0.0;

        for i in 0..self.frame_num {
            let buf_size = (self.frame_size[i].0 * self.frame_size[i].1 +
                            ((self.frame_size[i].0 + 1) / 2) *
                            ((self.frame_size[i].1 + 1) / 2) * 2) as
                           usize;
            let mut input1_buf = vec![0u8; buf_size];
            let mut input2_buf = vec![0u8; buf_size];

            let bytes_read1 = f1.read(&mut input1_buf).unwrap();
            if bytes_read1 != buf_size {
                return Err(Error::new(ErrorKind::Other, "bytes read1 is not expected ..."));
            }
            let bytes_read2 = f2.read(&mut input2_buf).unwrap();
            if bytes_read2 != buf_size {
                return Err(Error::new(ErrorKind::Other, "bytes read2 is not expected ..."));
            }

            let mut mse_y = 0.0f64;
            let mut mse_u = 0.0f64;
            let mut mse_v = 0.0f64;

            for j in 0..(self.frame_size[i].0 * self.frame_size[i].1) {
                let org = input1_buf[j as usize] as f64;
                let rec = input2_buf[j as usize] as f64;
                mse_y += (org - rec) * (org - rec);
            }
            for j in (self.frame_size[i].0 * self.frame_size[i].1)..
                     (self.frame_size[i].0 * self.frame_size[i].1 +
                      ((self.frame_size[i].0 + 1) / 2) * ((self.frame_size[i].1 + 1) / 2)) {
                let org = input1_buf[j as usize] as f64;
                let rec = input2_buf[j as usize] as f64;
                mse_u += (org - rec) * (org - rec);
            }
            for j in (self.frame_size[i].0 * self.frame_size[i].1 +
                      ((self.frame_size[i].0 + 1) / 2) * ((self.frame_size[i].1 + 1) / 2))..
                     (self.frame_size[i].0 * self.frame_size[i].1 +
                      ((self.frame_size[i].0 + 1) / 2) * ((self.frame_size[i].1 + 1) / 2) * 2) {
                let org = input1_buf[j as usize] as f64;
                let rec = input2_buf[j as usize] as f64;
                mse_v += (org - rec) * (org - rec);
            }

            mse_y /= (self.frame_size[i].0 * self.frame_size[i].1) as f64;
            mse_u /= (((self.frame_size[i].0 + 1) / 2) * ((self.frame_size[i].1 + 1) / 2)) as f64;
            mse_v /= (((self.frame_size[i].0 + 1) / 2) * ((self.frame_size[i].1 + 1) / 2)) as f64;

            let psnr_y = 10.0f64 * ((255.0f64 * 255.0f64) / mse_y).log10();
            let psnr_u = 10.0f64 * ((255.0f64 * 255.0f64) / mse_u).log10();
            let psnr_v = 10.0f64 * ((255.0f64 * 255.0f64) / mse_v).log10();
            let psnr = (4.0f64 * psnr_y + psnr_u + psnr_v) / 6.0f64;

            total_psnr_y += psnr_y;
            total_psnr_u += psnr_u;
            total_psnr_v += psnr_v;
            total_psnr += psnr;

            println!("Frame {:04}: PSNR_Y:{:2.2}, PSNR_U:{:2.2}, PSNR_V:{:2.2}, PSNR:{:2.2}",
                     i,
                     psnr_y,
                     psnr_u,
                     psnr_v,
                     psnr);
        }

        total_psnr_y /= self.frame_num as f64;
        total_psnr_u /= self.frame_num as f64;
        total_psnr_v /= self.frame_num as f64;
        total_psnr /= self.frame_num as f64;

        println!("=================================================================");
        println!("Total {:04}: PSNR_Y:{:2.2}, PSNR_U:{:2.2}, PSNR_V:{:2.2}, PSNR:{:2.2}\n",
                 self.frame_num,
                 total_psnr_y,
                 total_psnr_u,
                 total_psnr_v,
                 total_psnr);

        Ok(())
    }
}
