# mux-rust #

An implementation of the [Mux protocol](https://twitter.github.io/finagle/guide/Protocols.html#mux) in Rust.

Currently, this only includes a Reader and Writer for mux messages.

## Building ##

Use [cargo](https://crates.io/install):

    $ cargo build

## Running examples ##

    $ cargo test
    ...
    $ target/examples/server 
    serving on 0.0.0.0:6666
    0 rps
    0 rps
    -- 1: connected: Ok(127.0.0.1:56624)
    -- 0: connected: Ok(127.0.0.1:56625)
    -- 2: connected: Ok(127.0.0.1:56626)
    5875 rps
    16329 rps

    $ target/example/client
    :; target/examples/client 
    0 rps
    -- 0: connected: Ok(127.0.0.1:6666)
    -- 1: connected: Ok(127.0.0.1:6666)
    -- 2: connected: Ok(127.0.0.1:6666)
    17128 rps
    16152 rps


## Todo ##

* Attach to a client and server.
