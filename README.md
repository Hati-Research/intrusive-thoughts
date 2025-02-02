# Intrusive thoughts

> I've been having some intrusive thoughts - that while async embedded Rust is great,
it could also be better, more transparent and best practices should be documented.

This project serves two main purposes:

* To demystify some parts of the current embedded Rust ecosystem and
provide example solutions to some pain points that exist today.
* To serve as a notebook for my ideas. Note that these are just ideas,
not a definitive source of truth.
These ideas may be presented in a very raw form and important parts may be missing.

> Read more in the [book](intrusive.hatiresearch.eu).

## Repository contents

* book: sources for the documentation mdbook
* liltcp: a toy async networking stack
* nostd-playground: raw experiments requiring to be run on bare-metal
* std-playground: experiments that can be run on a computer to speed up development
* test-tcp-server: server used in experiments with liltcp

## License

```license
MIT License

Copyright (c) 2025 Hati Research, Matous Hybl

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
