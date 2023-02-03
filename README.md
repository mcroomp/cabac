# cabac
Context-adaptive binary arithmetic coding (CABAC) encoder / decoder in Rust

The coder is designed to encode binary values in an efficient manner, taking into account the
bits that were previously seen. The previous state is stored in the Context object,
which is updated each time a bit is seen. Normally multi-bit symbols are converted
to binary form, and each bit is assigned a corresponding bin (which gets its own context). 
Bits that are expected to be random can use the "bypass" mode, which are very efficiently
added to the bitstream without a context. 

In order to decode the bistream, the same contexts need to be used in the exact same sequence
or you will get back garbage. This also include any bypass bits that must also be read in the
same exact order.

---

There are two encoders included: the h264/h265 CABAC which uses a 6 bit state to track previously
seen bits, and the VP8 CABAC which uses a 16-bit state to track what it has seen.