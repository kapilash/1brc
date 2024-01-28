#![feature(portable_simd)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::fs::File;
use std::io::Read;
use std::simd;
use std::simd::cmp::SimdPartialEq;
use memchr::memchr;

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

fn simd_benchmark(c: &mut Criterion) {
    let mut f = File::open("measurements-mini.txt").unwrap();
    let mut buffer = Vec::new();
    let mut group = c.benchmark_group("indexof");
    f.read_to_end(&mut buffer).unwrap();
    group.bench_with_input("base_line", &buffer, |b, i| b.iter(|| base_line(black_box(&buffer[..]))));
    group.bench_with_input("using_memchr", &buffer, |b, i| b.iter(|| using_memchr(black_box(&buffer[..]))));
    group.bench_with_input("using_simd", &buffer, |b, i| b.iter(|| using_simd(black_box(&buffer[..]))));
    group.finish();
}
criterion_group!(benches, simd_benchmark );
criterion_main!(benches);
