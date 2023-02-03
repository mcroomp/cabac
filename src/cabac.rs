use std::io::Result;

/// implementation of a context aware binary arithmetic encoder
pub trait CabacWriter<Context> {
    /// write using bypass bin for bits that aren't worth encoding
    fn put_bypass(&mut self, bin_value: bool) -> Result<()>;

    /// write bits using given context for probability
    fn put(&mut self, value: bool, cur_ctx: &mut Context) -> Result<()>;

    /// flush any remaining state
    fn finish(&mut self) -> Result<()>;
}

/// implementation of a context aware binary arithmetic decoder
pub trait CabacReader<Context> {
    /// read from bypass bin
    fn get_bypass(&mut self) -> Result<bool>;

    /// read using given context for probability
    fn get(&mut self, cur_ctx: &mut Context) -> Result<bool>;
}
