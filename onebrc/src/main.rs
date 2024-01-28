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
use rayon::prelude::*;
use std::time::Instant;
use memchr::memchr;
use ahash::AHashMap; 
use std::simd;
use std::simd::cmp::SimdPartialEq;

#[derive(Clone)]
struct Weather {
    min_temp : i16,
    max_temp : i16,
    net_temp : i32,
    count    : u32
}

impl Weather {
    fn new(temperature : i16) -> Weather {
        let min_temp = temperature;
        let max_temp = temperature;
        let net_temp = i32::from(temperature);
        let count = 1;
        Weather { min_temp, max_temp, net_temp, count}
    }

    fn add(&mut self, temperature : i16) {
        self.min_temp = std::cmp::min(self.min_temp, temperature);
        /*unsafe { 
            core::intrinsics::atomic_max_seqcst(&mut self.max_temp, temperature);
            core::intrinsics::atomic_min_seqcst(&mut self.min_temp, temperature);
        }*/
        self.max_temp = std::cmp::max(self.max_temp, temperature);
        self.net_temp +=  i32::from(temperature); 
        self.count +=  1;
    }

    fn add_other(&mut self, other:& Weather) {
        self.min_temp = std::cmp::min(self.min_temp, other.min_temp);
        /*unsafe { 
            core::intrinsics::atomic_max_seqcst(&mut self.max_temp, other.max_temp);
            core::intrinsics::atomic_min_seqcst(&mut self.min_temp, other.min_temp);
        }*/
        self.max_temp = std::cmp::max(self.max_temp, other.max_temp);
        self.net_temp += other.net_temp;
        self.count += other.count;
    }
}

impl fmt::Display for Weather {
    fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
        let mint = f32::from(self.min_temp) / 10.0;
        let maxt = f32::from(self.max_temp) / 10.0;
        let avgt = f64::from(self.net_temp) / (10.0 * f64::from(self.count));
        write!(f, "{:.1}/{:.1}/{:.1}", mint, avgt, maxt)
    }
}

struct WeatherBatch {
    table: HashMap<String, Weather>
}

#[inline(always)]
fn semicolon_pos(bytes:&[u8]) -> usize {
   if bytes.len() < 64 {
       return memchr(b';', bytes).unwrap();
   }
   /*else if bytes.len() < 64 {
       let smd = simd::u8x32::from_slice(bytes);
       let semismd = simd::u8x32::splat(b';');
       let mask = smd.simd_eq(semismd);
       if let Some(pos) = mask.first_set() {
           pos
       } else{
           32 +     memchr(b';', &bytes[64..]).unwrap() 
       }

   }*/
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
       64 + memchr(b';', &bytes[64..]).unwrap()
   }
   else {
       memchr(b';', &bytes[64..]).unwrap() + 64  
   }
}

impl WeatherBatch {
    fn append_to(&self, result:&mut HashMap<String, Weather>) {
        for (key,val) in self.table.iter() {
            match result.get_mut(key) {
                Some(v) => v.add_other(val),
                None   => {result.insert(key.to_string(), val.clone()); }
            }
        }
    }

    fn new(bytes:&[u8]) -> WeatherBatch {
        let mut table1 : AHashMap<&[u8], Weather> = AHashMap::new();
        table1.reserve(20000);
         let mut pos = 0;
         while pos < bytes.len() {
             let s = semicolon_pos(&bytes[pos..]); 
             let city = &bytes[pos..(pos+s)];
             pos += s ;
             pos += 1;
             let mut sign:i16 = 1;
             if bytes[pos] == 45 {
                 sign = -1;
                 pos += 1;
             }
             let mut temperature : i16 = 0;
             while pos < bytes.len()  && bytes[pos] != 10 {
                 if bytes[pos] != 46  {
                     let curr = i16::from(bytes[pos] - 48);
                     temperature = temperature * 10 + curr;
                 }
                 pos += 1;
             }
             pos += 1;
             match table1.get_mut(&city) {
                 Some(v) => {v.add(temperature * sign); },
                 None => { table1.insert(city, Weather::new(temperature*sign)); }
             }
         }
         let mut table = HashMap::new();
         for (key,val) in table1.iter() {
             table.insert(String::from_utf8(key.to_vec()).unwrap(), val.clone());
         }
         WeatherBatch { table }
    }
}

fn next_end(file:&mut File, seek_position: u64, buffer:&mut [u8]) -> std::io::Result<u64> {
    file.seek(SeekFrom::Start(seek_position))?;
    file.read_exact(buffer)?;
    let mut pos = 0;
    while pos < 128 {
        if buffer[pos] == 10 {
            break;
        }
        pos += 1;
    }
    Ok(seek_position + u64::try_from(pos).unwrap())
}

fn chunk_sizes(file_name: &str, chunk_count: u64) -> std::io::Result<Vec<(u64, u64)>> {
    let mut result = Vec::new();
    let mut file = File::open(file_name)?;
    let file_size = file.metadata()?.len();
    let chunk_size : u64 = file_size / chunk_count ;
    let mut buffer = vec![0; 128];
    let mut prev_end = next_end(&mut file, chunk_size, &mut buffer)?;
    result.push((0, prev_end));
    for _i in 1..(chunk_count-1) {
         let ne = next_end(&mut file, prev_end + chunk_size, &mut buffer).unwrap();
         result.push((prev_end + 1, ne));
         prev_end = ne;
    }
    result.push((prev_end + 1, file_size));

    Ok(result)
}






fn main() -> std::io::Result<()> {
    let args:Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        let custom_error = Error::new(ErrorKind::Other, "Missing file name");
        return Err(custom_error);
    }
    let start = Instant::now();
    let chunk_count = 16;
    let chunk_regions = chunk_sizes(&args[0], chunk_count)?;
    let file = File::open(&args[0])?;
    let mmap = unsafe { Mmap::map(&file)? };

    let par_iter = chunk_regions.into_par_iter().map(|(s,e)| {
        let start = usize::try_from(s).unwrap();
        let end = usize::try_from(e).unwrap();
        WeatherBatch::new(&mmap[start..end])
        });
    let batches:Vec<WeatherBatch> = par_iter.collect();
    let mut result = HashMap::new();
    for b in batches {
        b.append_to(&mut result);
    }
    let duration = start.elapsed();
    let mut keys : Vec<String> = result.keys().cloned().collect();
    keys.sort_unstable();
    let mut is_first = true;
    print!("{{");
    for key in keys.iter() {
        if !is_first {
            print!(", ");
        }
        is_first = false;
        print!("{}={}", key, result.get(key).unwrap());
    }
    println!("}}");
    println!("Time elapsed = {:?}", duration);

    Ok(())
}
