# Core Ray Tracer

Rust `#![no_std]` raytracer using `&mut [u8]` for all allocations.
See the accompanying blog post:

https://matklad.github.io/2022/10/06/hard-mode-rust.html

```console
$ cargo r -r -p crt < scenes/utah.crt > out.ppm
```

![](https://user-images.githubusercontent.com/1711539/194287665-05583649-dcb0-4014-82b9-424f945e19a4.png)
