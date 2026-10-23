# Third-party notices

## QUIC Initial packet generator

`backend/src/quic_initial.lua` is adapted from `zemidala/nfqws2-ui` (`src/www/api.php`, audited at commit `631878b8b9a6908d106904c188a83afb7aa43dbf`).

## nfqws2-ui strategy candidate behavior

The bounded standard candidate families in `backend/src/strategy_picker.rs` are a Rust behavioral adaptation of the strategy catalog in `zemidala/nfqws2-ui`, audited at commit `631878b8b9a6908d106904c188a83afb7aa43dbf` (`src/www/api.php`, `STD_TLS`, `STD_HTTP`, `STD_QUIC`, and `catalogCandidates()`). They are validated against the installed Lua functions and blob files and are not a verbatim PHP port.

Copyright (c) 2026 zemidala

The upstream project is licensed under the MIT License:

> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

The audited `bol-van/zapret2` source, including `blockcheck2.sh`, is MIT licensed by the respective copyright holders. No shell fragments from `blockcheck2.sh` are included in this implementation.
