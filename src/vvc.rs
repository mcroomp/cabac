use std::io::{Result, Write, Read};

use crate::traits::{CabacReader, CabacWriter};

const BITS_IN_BYTE: i32 = 8;
const BITS_IN_LONG: i32 = 64;
const BITS_IN_LONG_MINUS_LAST_BYTE: i32 = BITS_IN_LONG - BITS_IN_BYTE;


#[derive(Debug)]
struct VVCContext
{
    a : u16,
    b : u16,
}

impl Default for VVCContext
{
    fn default() -> Self {
        Self { a: 35 << a_shift, b: 35 << b_shift }
    }
}

const a_shift : u8 = 3;
const b_shift : u8 = 7;


impl VVCContext
{
    fn update_true(&mut self)
    {
        self.a = self.a - (self.a >> a_shift) + (1023 >> a_shift);
        self.b = self.b - (self.b >> b_shift) + (16383 >> b_shift);
    }

    fn update_false(&mut self)
    {
        self.a = self.a - (self.a >> a_shift);
        self.b = self.b - (self.b >> b_shift);
    }

    fn get_probability(&self) -> u32
    {
        (u32::from(self.b) + u32::from(self.a) << 4) 
    }
}

#[test]
fn distrib()
{
    let mut ctx = VVCContext::default();
    println!("{0:?}", ctx);

    for i in 0..2000
    {
        ctx.update_true();
        ctx.update_false();
    }
    println!("alt={0:?} p={1}", ctx, ctx.get_probability());


    for i in 0..20000
    {
        ctx.update_true();
    }
    println!("t={0:?} p={1}", ctx, ctx.get_probability());

    for i in 0..20000
    {
        ctx.update_false();
    }
    println!("f={0:?} p={1}", ctx, ctx.get_probability());

}



