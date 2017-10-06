extern crate rsplit;

use std::env;
use std::process;
use rsplit::yuv::Yuv;
use rsplit::ivf::Ivf;
use rsplit::webm::Webm;
use rsplit::psnr::Psnr;
use rsplit::bin::Bin;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("too less arguments: {}", args.len());
        println!("Usage: rsplit bin|ivf|psnr|webm|yuv ...");
    } else {
        if args[1] == "yuv" {
            let yuv = Yuv::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                Yuv::helper();
                process::exit(1);
            });

            if let Err(err) = yuv.run() {
                println!("{}", err);
            }
        } else if args[1] == "psnr" {
            let psnr = Psnr::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                Psnr::helper();
                process::exit(1);
            });

            if let Err(err) = psnr.run() {
                println!("{}", err);
            }
        } else if args[1] == "ivf" {
            let ivf = Ivf::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                Ivf::helper();
                process::exit(1);
            });

            if let Err(err) = ivf.run() {
                println!("{}", err);
            }
        } else if args[1] == "bin" {
            let bin = Bin::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                Bin::helper();
                process::exit(1);
            });

            if let Err(err) = bin.run() {
                println!("{}", err);
            }
        } else if args[1] == "webm" {
            let webm = Webm::new(&args).unwrap_or_else(|err| {
                println!("Problem parsing arguments: {}", err);
                Webm::helper();
                process::exit(1);
            });

            if let Err(err) = webm.run() {
                println!("{}", err);
            }
        } else {
            println!("unsupported split {} mode", args[1]);
            println!("Usage: rsplit bin|ivf|psnr|webm|yuv ...");
        }
    }
}
