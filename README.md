# Safe Context-adaptive binary arithmetic coding (CABAC) encoder / decoder in Rust
Implementation of CABAC using H.264/265, VP8 and rANS encoders. 

The coder is designed to encode binary values in an efficient manner, taking into account the
bits that were previously seen. The previous state is stored in the Context object,
which is updated each time a bit is seen. Normally multi-bit symbols are converted
to binary form, and each bit is assigned a corresponding bin (which gets its own context). 
Bits that are expected to be random can use the "bypass" mode, which are very efficiently
added to the bitstream without a context. 

In order to decode the bistream, the same contexts need to be used in the exact same sequence
or you will get back garbage. This also include any bypass bits that must also be read in the
same exact order.

There are three encoders included: 
- h264/h265 CABAC which uses a 6 bit state to track previously
- VP8 CABAC which uses a 16-bit state to track what it has seen.
- rANS encoder (based on ryg_rans and dropbox/lepton) that uses the VP8 state to track probability

Performance notes:
- Criterion bench tests included
- No unsafe code
- rANS has not yet been significantly optimized although it outperforms the other encoders, encoding uses division although it could use an inverse multiple and decoding is done one value at a time, although it could be done in parallel.

Here is the relative performance (in microseconds, lower is better) for encoding, decoding as measured on Intel i9-12900K (compiled with -Ctarget-cpu=native):

| Encoder  | Read | Read bypass | Write |
| -------- | ---- | ----------- | ----- |
| H264/265 | 437  | 55          | 337   |
| VP8      | 288  | 149         | 219   |
| rANS     | 194  | 59          | 190   |

