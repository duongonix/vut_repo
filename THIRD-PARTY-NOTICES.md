# Third-Party Notices

Vut distributions statically link third-party Rust crates into the runtime
archives (`vut-core`, `vut-stdlib`) and the toolchain binaries (`vut`, `vpm`,
`vut-lsp`). This file lists those crates and reproduces the license texts that
require attribution.

Vut itself is licensed under the MIT License (see `LICENSE`). No third-party
component changes the licensing of Vut's own source.

The dependency inventory below is generated from `Cargo.lock`. Where a crate is
dual/multi-licensed, Vut uses it under the terms of one of the listed licenses
(typically MIT or Apache-2.0).

---

## License texts

### MIT License

```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Apache License 2.0

The full Apache License 2.0 text is available at
<https://www.apache.org/licenses/LICENSE-2.0>. Redistributions that include
Apache-2.0 components must retain the NOTICE file of any component that provides
one; the bundled crates do not ship separate NOTICE files.

### ISC License

```text
Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```

### BSD 3-Clause License

```text
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors
   may be used to endorse or promote products derived from this software
   without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

### Zlib License

```text
This software is provided 'as-is', without any express or implied warranty.
In no event will the authors be held liable for any damages arising from the
use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it freely,
subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim
   that you wrote the original software. If you use this software in a product,
   an acknowledgment in the product documentation would be appreciated but is
   not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.
```

### 0BSD License

```text
Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```

### Unicode License v3

See <https://www.unicode.org/license.txt>.

### CC0 1.0 Universal

See <https://creativecommons.org/publicdomain/zero/1.0/legalcode>.

### Mozilla Public License 2.0

See <https://www.mozilla.org/en-US/MPL/2.0/>.

---

## Bundled dependency inventory (generated from `Cargo.lock`)
- `adler2 2.0.1` - 0BSD OR MIT OR Apache-2.0
- `allocator-api2 0.2.21` - MIT OR Apache-2.0
- `anstream 1.0.0` - MIT OR Apache-2.0
- `anstyle 1.0.14` - MIT OR Apache-2.0
- `anstyle-parse 1.0.0` - MIT OR Apache-2.0
- `anstyle-query 1.1.5` - MIT OR Apache-2.0
- `anstyle-wincon 3.0.11` - MIT OR Apache-2.0
- `anyhow 1.0.104` - MIT OR Apache-2.0
- `arbitrary 1.4.2` - MIT OR Apache-2.0
- `arrayvec 0.7.8` - MIT OR Apache-2.0
- `async-trait 0.1.92` - MIT OR Apache-2.0
- `atomic-waker 1.1.2` - Apache-2.0 OR MIT
- `auto_impl 1.3.0` - MIT OR Apache-2.0
- `aws-lc-rs 1.18.1` - ISC AND (Apache-2.0 OR ISC)
- `aws-lc-sys 0.45.0` - ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)
- `base64 0.22.1` - MIT OR Apache-2.0
- `bitflags 1.3.2` - MIT/Apache-2.0
- `bitflags 2.13.2` - MIT OR Apache-2.0
- `blake3 1.8.7` - CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception
- `block-buffer 0.10.4` - MIT OR Apache-2.0
- `bumpalo 3.20.3` - MIT OR Apache-2.0
- `bytes 1.12.1` - MIT
- `cc 1.4.5` - MIT OR Apache-2.0
- `cfg_aliases 0.2.2` - MIT
- `cfg-if 1.0.4` - MIT OR Apache-2.0
- `chacha20 0.10.2` - MIT OR Apache-2.0
- `clap 4.6.6` - MIT OR Apache-2.0
- `clap_builder 4.6.6` - MIT OR Apache-2.0
- `clap_derive 4.6.4` - MIT OR Apache-2.0
- `clap_lex 1.1.0` - MIT OR Apache-2.0
- `cmake 0.1.58` - MIT OR Apache-2.0
- `colorchoice 1.0.5` - MIT OR Apache-2.0
- `constant_time_eq 0.4.2` - CC0-1.0 OR MIT-0 OR Apache-2.0
- `core-foundation-sys 0.8.7` - MIT OR Apache-2.0
- `cpufeatures 0.2.17` - MIT OR Apache-2.0
- `cpufeatures 0.3.1` - MIT OR Apache-2.0
- `cranelift-assembler-x64 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-assembler-x64-meta 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-bforest 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-bitset 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-codegen 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-codegen-meta 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-codegen-shared 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-control 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-entity 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-frontend 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-isle 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-module 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-object 0.135.2` - Apache-2.0 WITH LLVM-exception
- `cranelift-srcgen 0.135.2` - Apache-2.0 WITH LLVM-exception
- `crc32fast 1.5.1` - MIT OR Apache-2.0
- `crossbeam-deque 0.8.8` - MIT OR Apache-2.0
- `crossbeam-epoch 0.9.21` - MIT OR Apache-2.0
- `crossbeam-utils 0.8.23` - MIT OR Apache-2.0
- `crypto-common 0.1.7` - MIT OR Apache-2.0
- `dashmap 5.5.3` - MIT
- `derive_arbitrary 1.4.2` - MIT OR Apache-2.0
- `digest 0.10.7` - MIT OR Apache-2.0
- `dirs 6.0.0` - MIT OR Apache-2.0
- `dirs-sys 0.5.0` - MIT OR Apache-2.0
- `displaydoc 0.2.7` - MIT OR Apache-2.0
- `dunce 1.0.5` - CC0-1.0 OR MIT-0 OR Apache-2.0
- `either 1.18.0` - MIT OR Apache-2.0
- `equivalent 1.0.2` - Apache-2.0 OR MIT
- `errno 0.3.14` - MIT OR Apache-2.0
- `filetime 0.2.29` - MIT/Apache-2.0
- `find-msvc-tools 0.1.12` - MIT OR Apache-2.0
- `flate2 1.1.10` - MIT OR Apache-2.0
- `fnv 1.0.7` - Apache-2.0 / MIT
- `foldhash 0.2.0` - Zlib
- `form_urlencoded 1.2.2` - MIT OR Apache-2.0
- `fs_extra 1.3.0` - MIT
- `fs2 0.4.3` - MIT/Apache-2.0
- `futures 0.3.34` - MIT OR Apache-2.0
- `futures-channel 0.3.34` - MIT OR Apache-2.0
- `futures-core 0.3.34` - MIT OR Apache-2.0
- `futures-io 0.3.34` - MIT OR Apache-2.0
- `futures-macro 0.3.34` - MIT OR Apache-2.0
- `futures-sink 0.3.34` - MIT OR Apache-2.0
- `futures-task 0.3.34` - MIT OR Apache-2.0
- `futures-util 0.3.34` - MIT OR Apache-2.0
- `generic-array 0.14.7` - MIT
- `getrandom 0.2.17` - MIT OR Apache-2.0
- `getrandom 0.4.3` - MIT OR Apache-2.0
- `gimli 0.33.0` - MIT OR Apache-2.0
- `h2 0.4.19` - MIT
- `hashbrown 0.14.5` - MIT OR Apache-2.0
- `hashbrown 0.16.1` - MIT OR Apache-2.0
- `hashbrown 0.17.1` - MIT OR Apache-2.0
- `heck 0.5.0` - MIT OR Apache-2.0
- `http 1.5.0` - MIT OR Apache-2.0
- `http-body 1.1.0` - MIT
- `http-body-util 0.1.5` - MIT
- `httparse 1.10.1` - MIT OR Apache-2.0
- `hyper 1.11.1` - MIT
- `hyper-rustls 0.27.9` - Apache-2.0 OR ISC OR MIT
- `hyper-util 0.1.20` - MIT
- `icu_collections 2.3.0` - Unicode-3.0
- `icu_locale_core 2.3.0` - Unicode-3.0
- `icu_normalizer 2.3.0` - Unicode-3.0
- `icu_normalizer_data 2.3.0` - Unicode-3.0
- `icu_properties 2.3.0` - Unicode-3.0
- `icu_properties_data 2.3.0` - Unicode-3.0
- `icu_provider 2.3.1` - Unicode-3.0
- `idna 1.1.0` - MIT OR Apache-2.0
- `idna_adapter 1.2.2` - Apache-2.0 OR MIT
- `indexmap 2.14.2` - Apache-2.0 OR MIT
- `ipnet 2.12.2` - MIT OR Apache-2.0
- `is_terminal_polyfill 1.70.2` - MIT OR Apache-2.0
- `itoa 1.0.18` - MIT OR Apache-2.0
- `jobserver 0.1.35` - MIT OR Apache-2.0
- `js-sys 0.3.105` - MIT OR Apache-2.0
- `libc 0.2.189` - MIT OR Apache-2.0
- `libm 0.2.16` - MIT
- `libredox 0.1.24` - MIT
- `linux-raw-sys 0.12.1` - Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `litemap 0.8.3` - Unicode-3.0
- `lock_api 0.4.14` - MIT OR Apache-2.0
- `log 0.4.34` - MIT OR Apache-2.0
- `lru-slab 0.1.2` - MIT OR Apache-2.0 OR Zlib
- `lsp-types 0.94.1` - MIT
- `memchr 2.8.3` - Unlicense OR MIT
- `miniz_oxide 0.9.1` - MIT OR Zlib OR Apache-2.0
- `mio 1.2.3` - MIT
- `ntapi 0.4.3` - Apache-2.0 OR MIT
- `object 0.39.1` - Apache-2.0 OR MIT
- `once_cell 1.21.4` - MIT OR Apache-2.0
- `once_cell_polyfill 1.70.2` - MIT OR Apache-2.0
- `option-ext 0.2.0` - MPL-2.0
- `parking_lot_core 0.9.12` - MIT OR Apache-2.0
- `percent-encoding 2.3.2` - MIT OR Apache-2.0
- `pin-project 1.1.13` - Apache-2.0 OR MIT
- `pin-project-internal 1.1.13` - Apache-2.0 OR MIT
- `pin-project-lite 0.2.17` - Apache-2.0 OR MIT
- `pkg-config 0.3.34` - MIT OR Apache-2.0
- `potential_utf 0.1.6` - Unicode-3.0
- `proc-macro2 1.0.107` - MIT OR Apache-2.0
- `quinn 0.11.11` - MIT OR Apache-2.0
- `quinn-proto 0.11.17` - MIT OR Apache-2.0
- `quinn-udp 0.5.15` - MIT OR Apache-2.0
- `quote 1.0.47` - MIT OR Apache-2.0
- `r-efi 6.0.0` - MIT OR Apache-2.0 OR LGPL-2.1-or-later
- `rand 0.10.2` - MIT OR Apache-2.0
- `rand_core 0.10.1` - MIT OR Apache-2.0
- `rand_pcg 0.10.2` - MIT OR Apache-2.0
- `rayon 1.12.0` - MIT OR Apache-2.0
- `rayon-core 1.13.0` - MIT OR Apache-2.0
- `redox_syscall 0.5.18` - MIT
- `redox_users 0.5.2` - MIT
- `regalloc2 0.15.2` - Apache-2.0 WITH LLVM-exception
- `reqwest 0.12.28` - MIT OR Apache-2.0
- `ring 0.17.14` - Apache-2.0 AND ISC
- `rustc-hash 2.1.3` - Apache-2.0 OR MIT
- `rustix 1.1.5` - Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `rustls 0.23.44` - Apache-2.0 OR ISC OR MIT
- `rustls-pki-types 1.15.1` - MIT OR Apache-2.0
- `rustls-webpki 0.103.15` - ISC
- `rustversion 1.0.23` - MIT OR Apache-2.0
- `ryu 1.0.23` - Apache-2.0 OR BSL-1.0
- `scopeguard 1.2.0` - MIT OR Apache-2.0
- `semver 1.0.28` - MIT OR Apache-2.0
- `serde 1.0.229` - MIT OR Apache-2.0
- `serde_core 1.0.229` - MIT OR Apache-2.0
- `serde_derive 1.0.229` - MIT OR Apache-2.0
- `serde_json 1.0.151` - MIT OR Apache-2.0
- `serde_repr 0.1.21` - MIT OR Apache-2.0
- `serde_spanned 1.1.1` - MIT OR Apache-2.0
- `serde_urlencoded 0.7.1` - MIT/Apache-2.0
- `sha2 0.10.9` - MIT OR Apache-2.0
- `shlex 2.0.1` - MIT OR Apache-2.0
- `simd-adler32 0.3.10` - MIT
- `slab 0.4.12` - MIT
- `smallvec 1.16.0` - MIT OR Apache-2.0
- `socket2 0.6.5` - MIT OR Apache-2.0
- `stable_deref_trait 1.2.1` - MIT OR Apache-2.0
- `strsim 0.11.1` - MIT
- `subtle 2.6.1` - BSD-3-Clause
- `syn 2.0.119` - MIT OR Apache-2.0
- `syn 3.0.5` - MIT OR Apache-2.0
- `sync_wrapper 1.0.2` - Apache-2.0
- `synstructure 0.13.2` - MIT
- `sysinfo 0.31.4` - MIT
- `tar 0.4.46` - MIT OR Apache-2.0
- `target-lexicon 0.13.5` - Apache-2.0 WITH LLVM-exception
- `thiserror 2.0.20` - MIT OR Apache-2.0
- `thiserror-impl 2.0.20` - MIT OR Apache-2.0
- `tinystr 0.8.4` - Unicode-3.0
- `tinyvec 1.13.2` - Zlib OR Apache-2.0 OR MIT
- `tinyvec_macros 0.1.1` - MIT OR Apache-2.0 OR Zlib
- `tokio 1.53.1` - MIT
- `tokio-macros 2.7.2` - MIT
- `tokio-rustls 0.26.5` - MIT OR Apache-2.0
- `tokio-util 0.7.19` - MIT
- `toml 0.9.12+spec-1.1.0` - MIT OR Apache-2.0
- `toml_datetime 0.7.5+spec-1.1.0` - MIT OR Apache-2.0
- `toml_edit 0.23.10+spec-1.0.0` - MIT OR Apache-2.0
- `toml_parser 1.1.3+spec-1.1.0` - MIT OR Apache-2.0
- `toml_writer 1.1.2+spec-1.1.0` - MIT OR Apache-2.0
- `tower 0.4.13` - MIT
- `tower 0.5.3` - MIT
- `tower-http 0.6.11` - MIT
- `tower-layer 0.3.3` - MIT
- `tower-lsp 0.20.0` - MIT OR Apache-2.0
- `tower-lsp-macros 0.9.0` - MIT OR Apache-2.0
- `tower-service 0.3.3` - MIT
- `tracing 0.1.44` - MIT
- `tracing-attributes 0.1.31` - MIT
- `tracing-core 0.1.36` - MIT
- `try-lock 0.2.5` - MIT
- `typenum 1.20.1` - MIT OR Apache-2.0
- `unicode-ident 1.0.24` - (MIT OR Apache-2.0) AND Unicode-3.0
- `untrusted 0.9.0` - ISC
- `url 2.5.8` - MIT OR Apache-2.0
- `utf8_iter 1.0.4` - Apache-2.0 OR MIT
- `utf8parse 0.2.2` - Apache-2.0 OR MIT
- `version_check 0.9.5` - MIT/Apache-2.0
- `want 0.3.1` - MIT
- `wasi 0.11.1+wasi-snapshot-preview1` - Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
- `wasm-bindgen 0.2.128` - MIT OR Apache-2.0
- `wasm-bindgen-futures 0.4.78` - MIT OR Apache-2.0
- `wasm-bindgen-macro 0.2.128` - MIT OR Apache-2.0
- `wasm-bindgen-macro-support 0.2.128` - MIT OR Apache-2.0
- `wasm-bindgen-shared 0.2.128` - MIT OR Apache-2.0
- `wasm-streams 0.4.2` - MIT OR Apache-2.0
- `wasmtime-internal-core 48.0.2` - Apache-2.0 WITH LLVM-exception
- `web-sys 0.3.105` - MIT OR Apache-2.0
- `web-time 1.1.0` - MIT OR Apache-2.0
- `webpki-roots 1.0.9` - CDLA-Permissive-2.0
- `winapi 0.3.9` - MIT/Apache-2.0
- `winapi-i686-pc-windows-gnu 0.4.0` - MIT/Apache-2.0
- `winapi-x86_64-pc-windows-gnu 0.4.0` - MIT/Apache-2.0
- `windows 0.57.0` - MIT OR Apache-2.0
- `windows_aarch64_gnullvm 0.52.6` - MIT OR Apache-2.0
- `windows_aarch64_msvc 0.52.6` - MIT OR Apache-2.0
- `windows_i686_gnu 0.52.6` - MIT OR Apache-2.0
- `windows_i686_gnullvm 0.52.6` - MIT OR Apache-2.0
- `windows_i686_msvc 0.52.6` - MIT OR Apache-2.0
- `windows_x86_64_gnu 0.52.6` - MIT OR Apache-2.0
- `windows_x86_64_gnullvm 0.52.6` - MIT OR Apache-2.0
- `windows_x86_64_msvc 0.52.6` - MIT OR Apache-2.0
- `windows-core 0.57.0` - MIT OR Apache-2.0
- `windows-implement 0.57.0` - MIT OR Apache-2.0
- `windows-interface 0.57.0` - MIT OR Apache-2.0
- `windows-link 0.2.1` - MIT OR Apache-2.0
- `windows-result 0.1.2` - MIT OR Apache-2.0
- `windows-sys 0.52.0` - MIT OR Apache-2.0
- `windows-sys 0.61.2` - MIT OR Apache-2.0
- `windows-targets 0.52.6` - MIT OR Apache-2.0
- `winnow 0.7.15` - MIT
- `winnow 1.0.4` - MIT
- `writeable 0.6.4` - Unicode-3.0
- `xattr 1.6.1` - MIT OR Apache-2.0
- `yoke 0.8.3` - Unicode-3.0
- `yoke-derive 0.8.2` - Unicode-3.0
- `zerofrom 0.1.8` - Unicode-3.0
- `zerofrom-derive 0.1.7` - Unicode-3.0
- `zeroize 1.9.0` - Apache-2.0 OR MIT
- `zerotrie 0.2.5` - Unicode-3.0
- `zerovec 0.11.8` - Unicode-3.0
- `zerovec-derive 0.11.6` - Unicode-3.0
- `zip 2.4.2` - MIT
- `zlib-rs 0.6.8` - Zlib
- `zmij 1.0.23` - MIT
- `zopfli 0.8.3` - Apache-2.0
