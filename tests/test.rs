extern crate pi_lib;
extern crate pi_base;

use std::thread;
use std::io::Result;
use std::path::PathBuf;
use std::time::Duration;

use pi_base::pi_base_impl::STORE_TASK_POOL;
use pi_base::worker_pool::WorkerPool;
use pi_base::file::{AsyncFile, AsynFileOptions, WriteOptions};
use pi_base::util::{CompressLevel, compress, uncompress};

#[test]
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

#[test]
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