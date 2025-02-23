///! not implemented yet

#[derive(Debug)]
struct VVCContext {
    a: u16,
    b: u16,
}

impl Default for VVCContext {
    fn default() -> Self {
        Self {
            a: 35 << A_SHIFT,
            b: 35 << B_SHIFT,
        }
    }
}

const A_SHIFT: u8 = 3;
const B_SHIFT: u8 = 7;

impl VVCContext {
    fn update_true(&mut self) {
        self.a = self.a - (self.a >> A_SHIFT) + (1023 >> A_SHIFT);
        self.b = self.b - (self.b >> B_SHIFT) + (16383 >> B_SHIFT);
    }

    fn update_false(&mut self) {
        self.a = self.a - (self.a >> A_SHIFT);
        self.b = self.b - (self.b >> B_SHIFT);
    }

    fn get_probability(&self) -> u32 {
        u32::from(self.b) + u32::from(self.a) << 4
    }
}

#[test]
fn distrib() {
    let mut ctx = VVCContext::default();
    println!("{0:?}", ctx);

    for _i in 0..2000 {
        ctx.update_true();
        ctx.update_false();
    }
    println!("alt={0:?} p={1}", ctx, ctx.get_probability());

    for _i in 0..20000 {
        ctx.update_true();
    }
    println!("t={0:?} p={1}", ctx, ctx.get_probability());

    for _i in 0..20000 {
        ctx.update_false();
    }
    println!("f={0:?} p={1}", ctx, ctx.get_probability());
}
