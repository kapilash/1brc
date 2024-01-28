#![feature(portable_simd)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::fs::File;
use std::io::Read;
use memchr::memchr;

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

fn memchr_benchmark(c: &mut Criterion) {
    let mut f = File::open("measurements-mini.txt").unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    c.bench_function("base_line", |b| b.iter(|| using_memchr(black_box(&buffer[..]))));
}

criterion_group!(benches, memchr_benchmark );
criterion_main!(benches);
