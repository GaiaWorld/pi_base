#![feature(fnbox)]

extern crate npnc;
extern crate futures;

extern crate pi_lib;
extern crate pi_base;

use std::thread;
use std::sync::Arc;
use std::boxed::FnBox;
use std::path::PathBuf;
use std::time::Duration;
use std::result::Result as NormalResult;
use std::io::{Error, Result};

use futures::*;
use npnc::bounded::mpmc::{Producer, Consumer};

use pi_lib::atom::Atom;
use pi_base::task::TaskType;
use pi_base::pi_base_impl::{STORE_TASK_POOL, EXT_TASK_POOL, cast_ext_task};
use pi_base::worker_pool::WorkerPool;
use pi_base::file::{Shared, AsyncFile, AsynFileOptions, WriteOptions};
use pi_base::util::{CompressLevel, compress, uncompress};
use pi_base::future_pool::FutTaskPool;

// #[test]
fn test_lz4() {
    let string = String::from("asdfasdfpoiq;'wej(*(^$l;kKJ个）（&（）KLJ：LJLK：JLK：J：）（*）（*&（*&……&%……*UJK《JJL：HKLJHLKJHKJHKLHL：L：KHKJLGHYU……*&（%&……￥R%$#%$@#$%EDGFVNMLI_)(*%ERDHJGH0907886rtfhh)(&&$%$GFJHHJLJIOP(*jg%&$oujhlkjhnmgjhgljy98^&%^##$@$9878756543jkhmnbkmjou(*&(%^%$dfdhgjnlku^^%$$#%$egfcvmjhnl:kjo(&(&^%^%erfdgbh<jhkhiu^*(&*%&^%$^%ergfghghjlnbcvvxdasaew#$#%^*()_)(ytghjkl<mn%^%%#$%erdcffv:+{?}*&^%$#@!wsdefgw@@#$%^&JK;IO[IOU9078965(*&^%#$%$TGHJGFDFDGJHKIUTyghjkhty&ytkjljhgfghjhgfcvbnmrt(*&#*^$#^&*(*&^%$%&*(*&^%&^%$yhgffvbnmikyr$##%^&*(*&阿斯利康大家法律萨芬基本原理；声嘶力竭j8aslkjdfqpkmvpo09模压暗室逢灯阿斯顿发生地方东奔西走；辊；；基金会利用好吗，民");
    let buffer = string.as_bytes();
    let mut vec = Vec::with_capacity(0);
    vec.reserve(800);
    vec.resize(800, 0);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!buffer len: {}, vec len: {}, vec: {:?}", buffer.len(), vec.len(), vec);
    assert!(compress(buffer, &mut vec, CompressLevel::High).is_ok());
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!vec len: {}, vec: {:?}", vec.len(), vec);

    let mut vec_ = Vec::with_capacity(0);
    vec_.reserve(800);
    vec_.resize(800, 0);
    assert!(uncompress(&vec[..], &mut vec_).is_ok());
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!vec_ len: {}, vec_: {:?}", vec_.len(), vec_);
    assert!(String::from_utf8(vec_).ok().unwrap() == string);
}

// #[test]
fn test_file() {
	let worker_pool = Box::new(WorkerPool::new(10, 1024 * 1024, 10000));
    worker_pool.run(STORE_TASK_POOL.clone());

	let open = move |f0: Result<AsyncFile>| {
		assert!(f0.is_ok());
		let write = move |f1: AsyncFile, result: Result<()>| {
			assert!(result.is_ok());
			println!("!!!!!!write file");
			let write = move |_f3: AsyncFile, result: Result<()>| {
				assert!(result.is_ok());
				println!("!!!!!!write file by sync");
			};
			f1.write(WriteOptions::SyncAll(true), 8, Vec::from("Hello World!!!!!!######你好 Rust\nHello World!!!!!!######你好 Rust\nHello World!!!!!!######你好 Rust\n".as_bytes()), Box::new(write));
		};
		f0.ok().unwrap().write(WriteOptions::Flush, 0, vec![], Box::new(write));
	};
	AsyncFile::open(PathBuf::from(r"foo.txt"), AsynFileOptions::ReadWrite(1), Box::new(open));
	thread::sleep(Duration::from_millis(5000));

	let open = move |f0: Result<AsyncFile>| {
		assert!(f0.is_ok());
		println!("!!!!!!open file, symlink: {}, file: {}, only_read: {}, size: {}, time: {:?}", 
			f0.as_ref().ok().unwrap().is_symlink(), f0.as_ref().ok().unwrap().is_file(), f0.as_ref().ok().unwrap().is_only_read(), 
			f0.as_ref().ok().unwrap().get_size(), 
			(f0.as_ref().ok().unwrap().get_modified_time(), f0.as_ref().ok().unwrap().get_accessed_time(), f0.as_ref().ok().unwrap().get_created_time()));
		let read = move |f1: AsyncFile, result: Result<Vec<u8>>| {
			assert!(result.is_ok());
			println!("!!!!!!read file1, result: {:?}", result.ok().unwrap());
			let read = move |f3: AsyncFile, result: Result<Vec<u8>>| {
				assert!(result.is_ok());
				println!("!!!!!!read file3, result: {:?}", String::from_utf8(result.ok().unwrap()).unwrap_or("invalid utf8 string".to_string()));
				let read = move |f4: AsyncFile, result: Result<Vec<u8>>| {
					assert!(result.is_ok());
					println!("!!!!!!read file4, result: {:?}", String::from_utf8(result.ok().unwrap()).unwrap_or("invalid utf8 string".to_string()));
					let read = move |f7: AsyncFile, result: Result<Vec<u8>>| {
						assert!(result.is_ok());
						println!("!!!!!!read file7, result: {:?}", String::from_utf8(result.ok().unwrap()).unwrap_or("invalid utf8 string".to_string()));
						let read = move |f11: AsyncFile, result: Result<Vec<u8>>| {
							assert!(result.is_ok());
							println!("!!!!!!read file11, result: {:?}", String::from_utf8(result.ok().unwrap()).unwrap_or("invalid utf8 string".to_string()));
							let read = move |_f13: AsyncFile, result: Result<Vec<u8>>| {
								assert!(result.is_ok());
								println!("!!!!!!read file13, result: {:?}", String::from_utf8(result.ok().unwrap()).unwrap_or("invalid utf8 string".to_string()));
								
							};
							f11.read(0, 1000, Box::new(read));
						};
						f7.read(0, 34, Box::new(read));
					};
					f4.read(0, 32, Box::new(read));
				};
				f3.read(0, 16, Box::new(read));
			};
			f1.read(0, 10, Box::new(read));
		};
		f0.ok().unwrap().read(0, 0, Box::new(read));
	};
	AsyncFile::open(PathBuf::from(r"foo.txt"), AsynFileOptions::OnlyRead(1), Box::new(open));
	thread::sleep(Duration::from_millis(1000));

	let rename = move |from, to, result: Result<()>| {
		assert!(result.is_ok());
		println!("!!!!!!rename file, from: {:?}, to: {:?}", from, to);

		let remove = move |result: Result<()>| {
			assert!(result.is_ok());
			println!("!!!!!!remove file");
		};
		AsyncFile::remove(PathBuf::from(r"foo.swap"), Box::new(remove));
	};
	AsyncFile::rename(PathBuf::from(r"foo.txt"), PathBuf::from(r"foo.swap"), Box::new(rename));
	thread::sleep(Duration::from_millis(1000));
}

#[test]
fn test_shared_file() {
	let worker_pool = Box::new(WorkerPool::new(10, 1024 * 1024, 10000));
    worker_pool.run(STORE_TASK_POOL.clone());

	let open = move |f0: Result<AsyncFile>| {
		assert!(f0.is_ok());
		let shared = Arc::new(f0.ok().unwrap());
		let f0 = shared.clone();
		let f1 = shared.clone();
		let f3 = shared.clone();

		thread::spawn(move || {
			let write = move |shared0: Arc<AsyncFile>, result: Result<usize>| {
				assert!(result.is_ok() && result.ok() == Some(0));
				let write = move |_shared1: Arc<AsyncFile>, result: Result<usize>| {
					assert!(result.is_ok() && result.ok() == Some(105));
				};
				shared0.pwrite(WriteOptions::SyncAll(true), 8, Vec::from("Hello World!!!!!!######你好 Rust\nHello World!!!!!!######你好 Rust\nHello World!!!!!!######你好 Rust\n".as_bytes()), Box::new(write));
			};
			shared.pwrite(WriteOptions::Flush, 0, vec![], Box::new(write));
		});

		thread::spawn(move || {
			let write = move |f00: Arc<AsyncFile>, result: Result<usize>| {
				assert!(result.is_ok() && result.ok() == Some(0));
				let write = move |_f01: Arc<AsyncFile>, result: Result<usize>| {
					assert!(result.is_ok() && result.ok() == Some(137));
				};
				f00.pwrite(WriteOptions::SyncAll(true), 113, Vec::from("HelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHelloHello\n\n".as_bytes()), Box::new(write));
			};
			f0.pwrite(WriteOptions::Flush, 0, vec![], Box::new(write));
		});

		thread::spawn(move || {
			let read = move |f10: Arc<AsyncFile>, result: Result<Vec<u8>>| {
				assert!(result.is_ok() && result.ok().unwrap().len() == 250);
				let write = move |_f11: Arc<AsyncFile>, result: Result<usize>| {
					assert!(result.is_ok());
				};
				f10.pwrite(WriteOptions::SyncAll(true), 250, Vec::from("\nHello Rust0\n".as_bytes()), Box::new(write));
			};
			f1.pread(0, 250, Box::new(read));
		});

		thread::spawn(move || {
			let read = move |f11: Arc<AsyncFile>, result: Result<Vec<u8>>| {
				assert!(result.is_ok());
				println!("!!!!!!result: {:?}", result);
				let write = move |_f11: Arc<AsyncFile>, result: Result<usize>| {
					assert!(result.is_ok());
				};
				f11.pwrite(WriteOptions::SyncAll(true), 262, Vec::from("\nHello Rust1\n".as_bytes()), Box::new(write));
			};
			let mut buf = Vec::new();
			buf.resize(3, 255);
			f3.fpread(buf, 3, 0, 250, Box::new(read));
		});
	};
	AsyncFile::open(PathBuf::from(r"foo0.txt"), AsynFileOptions::ReadWrite(1), Box::new(open));
	thread::sleep(Duration::from_millis(5000));
	
	let rename = move |from, to, result: Result<()>| {
		assert!(result.is_ok());
		let remove = move |result: Result<()>| {
			assert!(result.is_ok());
		};
		AsyncFile::remove(PathBuf::from(r"foo0.swap"), Box::new(remove));
	};
	AsyncFile::rename(PathBuf::from(r"foo0.txt"), PathBuf::from(r"foo0.swap"), Box::new(rename));
	thread::sleep(Duration::from_millis(1000));
}

// #[test]
fn test_future() {
	let worker_pool = Box::new(WorkerPool::new(3, 1024 * 1024, 10000));
    worker_pool.run(EXT_TASK_POOL.clone());

	let pool = FutTaskPool::new(cast_ext_task);
	let callback = Box::new(move |executor: fn(TaskType, u64, Box<FnBox()>, Atom), 
									sender: Arc<Producer<NormalResult<usize, Error>>>, 
									receiver: Arc<Consumer<task::Task>>,
									uid: usize| {
		let func = Box::new(move || {
			thread::sleep_ms(10);
			match receiver.consume() {
				Err(e) => panic!("receive failed, {:?}", e),
				Ok(task) => {
					task.notify();
					assert!(sender.produce(Ok(uid)).is_ok());
				},
			}
		});
		executor(TaskType::Sync, 10000000, func, Atom::from("test future task"));
	});
	let mut future = pool.spawn(callback, 5000);
	let mut count = 0;
	loop {
		count += 1;
		thread::sleep_ms(1);
		match future.poll() {
			Ok(async) => {
				match async {
					Async::Ready(uid) => {
						assert!(uid == 0);
						break;
					}
					_ => continue,
				}
			},
			_ => continue,
		}
	}
}