# mux-rust #

[![Build Status](https://travis-ci.org/olix0r/mux-rust.svg?branch=master)](https://travis-ci.org/olix0r/mux-rust)

An implementation of the [Mux protocol](https://twitter.github.io/finagle/guide/Protocols.html#mux) in Rust.

This library includes a Reader and Writer for mux messages.  Due to the nature
of the Rust language, these APIs are extremely unstable and likely to change.

## Building ##

Use [cargo](https://crates.io/install):

    $ cargo build

### Building in Docker ###

Build an image with rust-nightly:

    $ docker build -t rust-nightly .

Run tests:

    $ docker run -t -i --rm=true -v $PWD:/src rust-nightly cargo test


## Running examples ##

Build example server and client:

    $ cargo test
    ...

Run a thread-per-connection server:

    $ target/examples/server
    serving on 0.0.0.0:6666
    -- 127.0.0.1:50047: connected
    5730 rps
    8627 rps

Run a single-threaded client:

    $ target/example/client
    -- 127.0.0.1:6666: connected: 127.0.0.1:50047
    0 rps
    8520 rps
    7994 rps

