# tet.rs

A falling blocks game made in Rust with [wgpu](https://www.github.com/gfx-rs/wgpu-rs).

## Features

* Highscore board with custom savefile format
* Custom image-based bitmap font rendering
* Crusty and hastily-typed code

## Building

Download the repository and run ```cargo build --release```. The output executable will be in `target/release/`. The game expects to find a `res` folder containing textures and shaders in its working directory.

Building requires shaderc to be available and properly configured in your system; check [shaderc-rs's repository](https://github.com/google/shaderc-rs) for more details. Building this also requires GLFW 3 to be installed in your machine; check [glfw-rs's repository](https://github.com/PistonDevelopers/glfw-rs) for more details.

## Note on code quality

The code quality in this project is intentionally left poor and should not be mimicked. This was hastily thrown together in a personal effort to learn how to use wgpu and as an exercise in discovering what a game made in Rust needs for infrastructure. As such, this code will eventually be iterated upon and made better, and eventually abstracted away into some sort of future framework.

Some plans in mind include abstracting away direct vertex manipulation into a generic Quad struct, alongside reducing the number of shader sets to one. The shaders are also currently written in GLSL, but the plan is for them to be eventually rewritten in WGSL, which is WebGPU's shading language. This has not been done yet because the specification changes frequently (as of writing this) and there are no simple resources for learning how to use it.

The text rendering system will also eventually be removed; the only reason why it exists is because the library I was planning on using, `wgpu-glyph`, had dependency conflicts with the `wgpu-rs` repository.

A reason why I am using wgpu straight from the git repository as a dependency is because wgpu 0.7.0, the most recent version at the time this was made, had a serious memory leak on its DX12 backend and other backends simply didn't work. Correct shader code was getting optimized away and data simply was not being passed around. These issues did not exist in the repo head.

## Copyright

This repository is licensed under the MIT license.
