use std::io::Cursor;

use cabac::{
    h265::{H265Context, H265Reader, H265Writer},
    traits::{CabacReader, CabacWriter},
    vp8::{VP8Context, VP8Reader, VP8Writer},
};

fn pattern(i: i32) -> bool {
    i % 111 == 0
}

const LOOP: i32 = 100 * 1024;

fn norm_vp8(print: bool) {
    let mut output = Vec::with_capacity(1000);
    {
        let mut writer = VP8Writer::new(&mut output).unwrap();
        let mut context = VP8Context::default();
        for i in 0..LOOP {
            writer.put(pattern(i), &mut context).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();
        let mut context = VP8Context::default();
        for i in 0..LOOP {
            assert_eq!(reader.get(&mut context).unwrap(), pattern(i));
        }
    }

    if print {
        println!("norm_vp8 = {0}", output.len() * 8);
    }
}

fn norm_h265(print: bool) {
    let mut output = Vec::with_capacity(1000);
    {
        let mut writer = H265Writer::new(&mut output);
        let mut context = H265Context::default();
        for i in 0..LOOP {
            writer.put(pattern(i), &mut context).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();
        let mut context = H265Context::default();
        for i in 0..LOOP {
            assert_eq!(reader.get(&mut context).unwrap(), pattern(i));
        }
    }

    if print {
        println!("norm_h265 = {0}", output.len() * 8);
    }
}

fn bypass_vp8(print: bool) {
    let mut output = Vec::with_capacity(1000);
    {
        let mut writer = VP8Writer::new(&mut output).unwrap();
        for i in 0..LOOP {
            writer.put_bypass(pattern(i)).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = VP8Reader::new(Cursor::new(&output)).unwrap();
        for i in 0..LOOP {
            assert_eq!(reader.get_bypass().unwrap(), pattern(i));
        }
    }

    if print {
        println!("bypass_vp8 = {0}", output.len() * 8);
    }
}

fn bypass_h265(print: bool) {
    let mut output = Vec::with_capacity(1000);
    {
        let mut writer = H265Writer::new(&mut output);
        for i in 0..LOOP {
            writer.put_bypass(pattern(i)).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = H265Reader::new(Cursor::new(&output)).unwrap();
        for i in 0..LOOP {
            assert_eq!(reader.get_bypass().unwrap(), pattern(i));
        }
    }

    if print {
        println!("bypass_h265 = {0}", output.len() * 8);
    }
}

fn main() {
    for i in 0..1024 {
        bypass_h265(i == 1023);
        bypass_vp8(i == 1023);
        norm_h265(i == 1023);
        norm_vp8(i == 1023);
    }
}
