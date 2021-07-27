# extern "rust"
With the introduction of [RTIC 0.6](https://github.com/rtic-rs/cortex-m-rtic) modularity is possible.

In previous versions of RTIC, all the firmware's source code had to live within a single 
module. This is disadvantagous for many reasons, including maintainability and organization.

In `main.rs`, you may observe like this
```rust
{{#include ../../src/main.rs:7:10}}
    // ...
{{#include ../../src/main.rs:66:69}}
```

## What is `extern "Rust"` doing here?`
Basically, this is a prototype using C++ terminology. We are declaring to the rust compiler that a 
function with this signature *exists*.  The actual implementation exists somewhere else.

This allows us to define the implementation of the function in another module, 
and bring that implementation into scope to satisfy this prototype.

## The benefit
As demonstrated, we are allowed to define the interrupt handler in a separate module.
This increases modularity and makes the firmware easier to write. 