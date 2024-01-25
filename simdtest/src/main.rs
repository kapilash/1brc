#![feature(portable_simd)]

use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::env;
use std::io::{Error, ErrorKind};
use std::io::SeekFrom;
use std::convert::TryFrom;
use std::fmt;
use std::collections::HashMap;
use memmap2::Mmap;
use std::time::Instant;
use memchr::memchr;
use std::simd;
use std::simd::cmp::SimdPartialEq;

fn base_line(bytes:&[u8]) -> (usize, i64) {
    let mut result = 0;
    let mut pos = 0;
    let mut net_temp = 0;
    while pos < bytes.len() {
        let mut s = 0;
        for i in 0..128 {
            if bytes[pos + i] == b';' {
                break;
            }
            s = s + 1;
        }
        result = result + s ;
        pos = pos + s ;
        pos = pos + 1;
        let mut sign:i16 = 1;
        if bytes[pos] == 45 {
            sign = -1;
            pos = pos + 1;
         }
         let mut temperature : i16 = 0;
         while pos < bytes.len()  && bytes[pos] != 10 {
             if bytes[pos] != 46  {
                 let curr = i16::from(bytes[pos] - 48);
                 temperature = temperature * 10 + curr;
             }
             pos = pos + 1;
         }
         net_temp = net_temp + i64::from(sign * temperature);
         pos = pos + 1;
    }
    (result, net_temp)
}

fn using_memchr(bytes:&[u8]) -> (usize, i64) {
    let mut result = 0;
    let mut pos = 0;
    let mut net_temp = 0;
    while pos < bytes.len() {
        let s = memchr(b';', &bytes[pos..]).unwrap();
        result = result + s;
        pos = pos + s ;
        pos = pos + 1;
        let mut sign:i16 = 1;
        if bytes[pos] == 45 {
            sign = -1;
            pos = pos + 1;
         }
         let mut temperature : i16 = 0;
         while pos < bytes.len()  && bytes[pos] != 10 {
             if bytes[pos] != 46  {
                 let curr = i16::from(bytes[pos] - 48);
                 temperature = temperature * 10 + curr;
             }
             pos = pos + 1;
         }
         net_temp = net_temp + i64::from(sign * temperature);
         pos = pos + 1;
    }
    (result, net_temp)
}

fn semicolon_pos(bytes:&[u8]) -> usize {
   if bytes.len() < 64 {
       return memchr(b';', &bytes).unwrap();
   }
   else /*if bytes.len() < 64*/ {
       let smd = simd::u8x32::from_slice(bytes);
       let semismd = simd::u8x32::splat(b';');
       let mask = smd.simd_eq(semismd);
       if let Some(pos) = mask.first_set() {
           pos
       } else{
           32 + semicolon_pos(&bytes[32..])
       }

   }
   /*
   let smd = simd::u8x64::from_slice(bytes);
   let semismd = simd::u8x64::splat(b';');
   let mask = smd.simd_eq(semismd);
   if let Some(pos) = mask.first_set() {
       return pos;
   } 
   if bytes.len() < 128 {
       let smd = simd::u8x64::from_slice(&bytes[64..]);
       let mask = smd.simd_eq(semismd);
       if let Some(pos) = mask.first_set() {
           return 64 + pos;
       }
       return 64 + semicolon_pos(&bytes[64..]);
   }
   else {
       memchr(b';', &bytes[64..]).unwrap() + 64  
   }*/
}

fn using_simd(bytes:&[u8]) -> (usize, i64) {
    let mut result = 0;
    let mut pos = 0;
    let mut net_temp = 0;
    while pos < bytes.len() {
        let s = semicolon_pos(&bytes[pos..]); 
        result = result + s;
        pos = pos + s ;
        pos = pos + 1;
        let mut sign:i16 = 1;
        if bytes[pos] == 45 {
            sign = -1;
            pos = pos + 1;
         }
         let mut temperature : i16 = 0;
         while pos < bytes.len()  && bytes[pos] != 10 {
             if bytes[pos] != 46  {
                 let curr = i16::from(bytes[pos] - 48);
                 temperature = temperature * 10 + curr;
             }
             pos = pos + 1;
         }
         net_temp = net_temp + i64::from(sign * temperature);
         pos = pos + 1;
    }
    (result, net_temp)
}

fn main() -> std::io::Result<()> {
    let args:Vec<String> = env::args().skip(1).collect();
    if args.len() < 1 {
        let custom_error = Error::new(ErrorKind::Other, "Missing file name");
        return Err(Error::from(custom_error));
    }
    let file = File::open(&args[0])?;
    let mmap = unsafe { Mmap::map(&file)? };
    let start = Instant::now();
    let (s,t) = base_line(&mmap);
    let duration = start.elapsed();
    println!("Baseline {}, {} in {:?} milliseconds", s, t, duration);
    let memchstart = Instant::now();
    let (s,t) = using_memchr(&mmap);
    let memch_duration = memchstart.elapsed();
    println!("memchar {}, {} in {:?} milliseconds", s, t, memch_duration);

    let simd_start = Instant::now();
    let (s,t) = using_simd(&mmap);
    let simd_duration = simd_start.elapsed();
    println!("simd {}, {} in {:?} milliseconds", s, t, simd_duration);
    Ok(())
}
