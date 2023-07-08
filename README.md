# LLVM Kaleidoscope Tutorial in Rust

This is a toy project to test LLVM and manipulate other tools.

It follows the tutorial: https://llvm.org/docs/tutorial/index.html
It is globally a direct port of the C++ code, with some adjustements.


[Inkwel](https://github.com/TheDan64/inkwell) is used for LLVM bindings in Rust.

Here what is currently done, with some differences with the original tutorial:

- Step1: Lexer
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl01.html

- Step 2: Parser and AST
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl02.html

- Step 3: Intermediate Representation (IR) code generation
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl03.html

- Step 4: JIT and Optimizer support
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl04.html
    - JIT: with LLVM 15 and Inkwell bindings, I could not reproduce exactly the
      code. But there was a way to easily create a JIT execution engine from a module.
    - It it not possible to redefine an existing function.

- Step 5: Control flow extension
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl05.html
    - Added a CLI option to disable optimization and observe result on IR

- Step 6: User-defined Operators
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl06.html
    - Add possibility of loading external scripts

- Step 7: Mutable variables
    - https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl07.html


## How to run

Launch:

    cargo run

Some options are available:

    cargo run -- --help

You can notably load files containing kaleido code:

    cargo run -- -f tests/scripts/mandelbrot.kaleido
    cargo run -- -f tests/scripts/fib.kaleido


## License

MIT License

Copyright (c) 2023 Vincent Hiribarren

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
