#![forbid(unsafe_code)]
#![forbid(trivial_numeric_casts)]
#![forbid(unused_qualifications)]
#![forbid(unused_results)]
#![forbid(unreachable_pub)]
#![forbid(deprecated_in_future)]

mod debug;
mod fpaq0;
mod fpaq0parallel;
mod h265;
pub mod perf;
mod rans32;
mod traits;
mod vp8;

pub use debug::{DebugReader, DebugWriter};
pub use fpaq0::{Fpaq0Decoder, Fpaq0Encoder};
pub use fpaq0parallel::{Fpaq0DecoderParallel, Fpaq0EncoderParallel};
pub use h265::{H265Reader, H265Writer};
pub use rans32::{RansReader32, RansWriter32};
pub use traits::{CabacReader, CabacWriter};
pub use vp8::{VP8Context, VP8Reader, VP8Writer};
