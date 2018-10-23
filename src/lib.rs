#![crate_type = "rlib"]
#![feature(asm)]
#![feature(libc)]
#![feature(fnbox)]
#![feature(drain_filter)]
#![feature(rustc_private)]
#![feature(type_ascription)]
#![feature(duration_extras)]
#![feature(slice_internals)]
#![feature(duration_from_micros)]
#![feature(integer_atomics)]
#![feature(extern_prelude)]

extern crate fnv;
extern crate core;
extern crate time;
extern crate rand;
extern crate libc;
extern crate npnc;
extern crate futures;
extern crate threadpool;

#[macro_use]
extern crate lazy_static;

extern crate lz4;
extern crate notify;

extern crate pi_lib;

pub mod pi_base_impl;
pub mod file;
pub mod fs_monitor;
pub mod future;
pub mod future_pool;
pub mod worker;
pub mod worker_pool;
pub mod task;
pub mod task_pool;
pub mod util;